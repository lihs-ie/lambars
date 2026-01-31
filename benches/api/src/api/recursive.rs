//! Recursive processing handlers using Trampoline for stack-safe recursion.
//!
//! This module demonstrates:
//! - `Trampoline`: Stack-safe recursive computation
//! - `Either`: Success/failure representation
//! - `PersistentHashSet`: Immutable visited node tracking
//! - `Semigroup`/`Monoid`: Statistics aggregation
//!
//! # lambars Features Demonstrated
//!
//! - **Trampoline**: Converts recursion into iteration for arbitrary depths
//! - **Either**: Type-safe error handling for cycle detection
//! - **Persistent Data Structures**: Immutable state management during recursion

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};

use super::json_buffer::JsonResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use lambars::control::{Either, Trampoline};
use lambars::persistent::PersistentHashSet;
use lambars::typeclass::Semigroup;

use super::error::{ApiErrorResponse, FieldError};
use super::handlers::AppState;
use crate::domain::{Task, TaskId, TaskStatus};

// =============================================================================
// DTOs
// =============================================================================

/// Query parameters for flatten-subtasks endpoint.
#[derive(Debug, Deserialize)]
pub struct FlattenSubtasksQuery {
    /// Maximum depth to flatten (default: 100, max: 10000).
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

const fn default_max_depth() -> usize {
    100
}

/// Maximum number of nodes to return in response (prevents huge payloads).
const MAX_RESPONSE_NODES: usize = 1000;

/// A flattened subtask with depth information.
#[derive(Debug, Clone, Serialize)]
pub struct FlattenedSubTask {
    pub id: String,
    pub title: String,
    pub depth: usize,
    /// Direct parent ID only (not full path) for smaller response size.
    pub parent_id: Option<String>,
}

/// Response for flatten-subtasks endpoint.
#[derive(Debug, Serialize)]
pub struct FlattenSubtasksResponse {
    pub task_id: String,
    pub flattened_subtasks: Vec<FlattenedSubTask>,
    pub total_count: usize,
    pub max_depth_reached: usize,
    pub trampoline_iterations: usize,
}

/// Request for resolve-dependencies endpoint.
#[derive(Debug, Deserialize)]
pub struct ResolveDependenciesRequest {
    /// Task IDs to resolve dependencies for.
    pub task_ids: Vec<String>,
}

/// Response for resolve-dependencies endpoint.
#[derive(Debug, Serialize)]
pub struct DependencyResolutionResponse {
    pub execution_order: Vec<String>,
    pub dependency_graph: HashMap<String, Vec<String>>,
    pub has_cycle: bool,
    pub cycle_path: Option<Vec<String>>,
    pub trampoline_iterations: usize,
}

/// Aggregated statistics for a task tree node.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AggregatedStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub completion_rate: f64,
    pub total_estimated_duration: u64,
}

impl Semigroup for AggregatedStats {
    #[allow(clippy::cast_precision_loss)]
    fn combine(self, other: Self) -> Self {
        let total = self.total_tasks + other.total_tasks;
        let completed = self.completed_tasks + other.completed_tasks;
        let rate = if total > 0 {
            completed as f64 / total as f64
        } else {
            0.0
        };
        Self {
            total_tasks: total,
            completed_tasks: completed,
            completion_rate: rate,
            total_estimated_duration: self.total_estimated_duration
                + other.total_estimated_duration,
        }
    }
}

/// Task node with aggregated statistics.
#[derive(Debug, Clone, Serialize)]
pub struct TaskNodeStats {
    pub task_id: String,
    pub title: String,
    pub stats: AggregatedStats,
    pub children: Vec<Self>,
}

/// Response for aggregate-tree endpoint.
#[derive(Debug, Serialize)]
pub struct TreeAggregationResponse {
    pub project_id: String,
    pub root_stats: AggregatedStats,
    pub tree: Vec<TaskNodeStats>,
    pub trampoline_iterations: usize,
    /// Task IDs that failed to load (empty if all tasks loaded successfully).
    pub failed_task_ids: Vec<String>,
}

/// Query parameters for aggregate-tree endpoint.
#[derive(Debug, Deserialize)]
pub struct AggregateTreeQuery {
    /// Maximum tree depth to aggregate (default: 10, max: 50).
    #[serde(default = "default_tree_depth")]
    pub max_depth: usize,
}

const fn default_tree_depth() -> usize {
    10
}

// =============================================================================
// Simulated Nested Structure for Demonstration
// =============================================================================

/// Simulated nested subtask for Trampoline demonstration.
#[derive(Debug, Clone)]
struct SimulatedSubTask {
    id: String,
    title: String,
    children: Vec<Self>,
}

impl SimulatedSubTask {
    fn generate_tree(
        depth: usize,
        breadth: usize,
        current_depth: usize,
        parent_id: &str,
    ) -> Vec<Self> {
        if current_depth >= depth {
            return Vec::new();
        }

        (0..breadth)
            .map(|i| {
                let id = format!("{parent_id}-{i}");
                let title = format!("Subtask at depth {current_depth}, index {i}");
                let children = Self::generate_tree(depth, breadth, current_depth + 1, &id);
                Self {
                    id,
                    title,
                    children,
                }
            })
            .collect()
    }
}

// =============================================================================
// GET /tasks/{id}/flatten-subtasks - Stack-safe subtask flattening
// =============================================================================

/// Flattens a task's subtask hierarchy using Trampoline for stack safety.
///
/// This handler demonstrates:
/// - **`Trampoline`**: Stack-safe recursive subtask flattening
///
/// The Trampoline pattern allows processing arbitrarily deep hierarchies
/// without risking stack overflow, by converting recursion into iteration.
///
/// # Path Parameters
///
/// - `id`: Task UUID
///
/// # Query Parameters
///
/// - `max_depth`: Maximum depth to flatten (default: 100, max: 10000)
///
/// # Errors
///
/// - `400 Bad Request`: Invalid depth parameter or task ID format
/// - `404 Not Found`: Task not found
#[allow(clippy::unused_async)]
pub async fn flatten_subtasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<FlattenSubtasksQuery>,
) -> Result<JsonResponse<FlattenSubtasksResponse>, ApiErrorResponse> {
    // Validate max_depth
    if query.max_depth > 10000 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "max_depth",
                "max_depth must be at most 10000",
            )],
        ));
    }

    // Parse task ID
    let task_id = Uuid::parse_str(&id).map_err(|_| {
        ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("id", "Invalid task ID format")],
        )
    })?;

    // Verify task exists
    let task = state
        .task_repository
        .find_by_id(&TaskId::from_uuid(task_id))
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found("Task not found"))?;

    // Generate simulated nested structure for demonstration
    // In production, this would be actual nested subtasks
    // Use a reasonable depth for simulation (up to 8 for demo purposes)
    // Breadth of 2 with depth 8 = 2^8 - 1 = 255 nodes (fast response)
    let simulated_depth = query.max_depth.min(8);
    let simulated_breadth = 2;
    let simulated_subtasks = SimulatedSubTask::generate_tree(
        simulated_depth,
        simulated_breadth,
        0,
        &task.task_id.to_string(),
    );

    // Use Trampoline for stack-safe flattening
    let (flattened, iterations, max_depth_reached) =
        flatten_subtasks_trampoline(&simulated_subtasks, query.max_depth);

    Ok(JsonResponse(FlattenSubtasksResponse {
        task_id: id,
        total_count: flattened.len(),
        flattened_subtasks: flattened,
        max_depth_reached,
        trampoline_iterations: iterations,
    }))
}

/// Pure: Flattens subtasks using Trampoline for stack safety.
///
/// Returns (flattened list, iteration count, max depth reached).
fn flatten_subtasks_trampoline(
    subtasks: &[SimulatedSubTask],
    max_depth: usize,
) -> (Vec<FlattenedSubTask>, usize, usize) {
    // Work queue item: (subtask owned, depth, parent_id)
    // We clone subtasks to own them, as Trampoline requires 'static lifetime
    #[derive(Clone)]
    struct WorkItem {
        subtask: SimulatedSubTask,
        depth: usize,
        parent_id: Option<String>,
    }

    fn process_work(
        work: Vec<WorkItem>,
        max_depth: usize,
        accumulated: Vec<FlattenedSubTask>,
        iterations: usize,
        max_depth_seen: usize,
    ) -> Trampoline<(Vec<FlattenedSubTask>, usize, usize)> {
        if work.is_empty() {
            return Trampoline::done((accumulated, iterations, max_depth_seen));
        }

        let mut acc = accumulated;
        let iter_count = iterations + 1;

        Trampoline::suspend(move || {
            let mut next_work = Vec::new();
            let mut current_max_depth = max_depth_seen;

            for item in work {
                // Skip if max depth exceeded or max nodes reached
                if item.depth > max_depth || acc.len() >= MAX_RESPONSE_NODES {
                    continue;
                }

                current_max_depth = current_max_depth.max(item.depth);

                let current_id = item.subtask.id.clone();

                // Add current subtask to result
                acc.push(FlattenedSubTask {
                    id: current_id.clone(),
                    title: item.subtask.title.clone(),
                    depth: item.depth,
                    parent_id: item.parent_id,
                });

                // Queue children for processing (with current node as their parent)
                for child in item.subtask.children {
                    next_work.push(WorkItem {
                        subtask: child,
                        depth: item.depth + 1,
                        parent_id: Some(current_id.clone()),
                    });
                }
            }

            process_work(next_work, max_depth, acc, iter_count, current_max_depth)
        })
    }

    // Clone subtasks to create initial work items (Trampoline requires 'static)
    let initial_work: Vec<WorkItem> = subtasks
        .iter()
        .map(|s| WorkItem {
            subtask: s.clone(),
            depth: 0,
            parent_id: None,
        })
        .collect();

    let trampoline = process_work(initial_work, max_depth, Vec::new(), 0, 0);
    trampoline.run()
}

// =============================================================================
// POST /tasks/resolve-dependencies - Topological sort with cycle detection
// =============================================================================

/// Resolves task dependencies using Trampoline-based topological sort.
///
/// This handler demonstrates:
/// - **`Trampoline`**: Stack-safe depth-first search
/// - **`Either`**: Type-safe cycle detection
/// - **`PersistentHashSet`**: Immutable visited state tracking
///
/// # Request Body
///
/// ```json
/// {
///   "task_ids": ["uuid-1", "uuid-2", "uuid-3"]
/// }
/// ```
///
/// # Errors
///
/// - `400 Bad Request`: Too many tasks or invalid task ID
/// - `404 Not Found`: Task not found
#[allow(clippy::unused_async)]
pub async fn resolve_dependencies(
    State(state): State<AppState>,
    Json(request): Json<ResolveDependenciesRequest>,
) -> Result<JsonResponse<DependencyResolutionResponse>, ApiErrorResponse> {
    // Validate task count
    if request.task_ids.len() > 1000 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new(
                "task_ids",
                "Cannot process more than 1000 tasks",
            )],
        ));
    }

    // Parse and validate task IDs
    let mut tasks: Vec<Task> = Vec::new();
    for task_id_str in &request.task_ids {
        let task_id = Uuid::parse_str(task_id_str).map_err(|_| {
            ApiErrorResponse::validation_error(
                "Validation failed",
                vec![FieldError::new(
                    "task_ids",
                    format!("Invalid task ID: {task_id_str}"),
                )],
            )
        })?;

        let task = state
            .task_repository
            .find_by_id(&TaskId::from_uuid(task_id))
            .await
            .map_err(ApiErrorResponse::from)?
            .ok_or_else(|| ApiErrorResponse::not_found(format!("Task not found: {task_id_str}")))?;

        tasks.push(task);
    }

    // Build simulated dependency graph for demonstration
    // In production, this would use actual task dependencies
    let dependency_graph = build_simulated_dependency_graph(&tasks);

    // Use Trampoline for stack-safe topological sort with cycle detection
    let (result, iterations) = topological_sort_trampoline(&dependency_graph);

    match result {
        Either::Left(cycle_path) => Ok(JsonResponse(DependencyResolutionResponse {
            execution_order: Vec::new(),
            dependency_graph: dependency_graph
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            has_cycle: true,
            cycle_path: Some(cycle_path),
            trampoline_iterations: iterations,
        })),
        Either::Right(order) => Ok(JsonResponse(DependencyResolutionResponse {
            execution_order: order,
            dependency_graph: dependency_graph
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            has_cycle: false,
            cycle_path: None,
            trampoline_iterations: iterations,
        })),
    }
}

/// Builds a simulated dependency graph for demonstration.
fn build_simulated_dependency_graph(tasks: &[Task]) -> HashMap<String, Vec<String>> {
    let mut graph = HashMap::new();

    // Create a simple linear dependency chain for demonstration
    // task_0 -> task_1 -> task_2 -> ...
    for (i, task) in tasks.iter().enumerate() {
        let task_id = task.task_id.to_string();
        let deps = if i > 0 {
            vec![tasks[i - 1].task_id.to_string()]
        } else {
            Vec::new()
        };
        graph.insert(task_id, deps);
    }

    graph
}

/// Pure: Performs topological sort using Trampoline.
///
/// Returns `Either::Left(cycle_path)` if cycle detected, `Either::Right(order)` otherwise.
///
/// This implementation uses a work-queue based approach to ensure true stack safety.
/// The Trampoline processes one step at a time, avoiding recursive `.run()` calls.
#[allow(clippy::items_after_statements)]
fn topological_sort_trampoline(
    graph: &HashMap<String, Vec<String>>,
) -> (Either<Vec<String>, Vec<String>>, usize) {
    use std::sync::Arc;

    // Work item represents a single step in DFS
    #[derive(Clone)]
    enum WorkItem {
        // Enter a node: check if visited/in_progress, then schedule deps and post-visit
        Enter { node: String, path: Vec<String> },
        // After all deps processed, mark as visited and add to result
        PostVisit { node: String },
    }

    // Shared graph reference to avoid cloning
    let graph = Arc::new(graph.clone());

    // State for the topological sort
    #[derive(Clone)]
    struct TopoState {
        visited: PersistentHashSet<String>,
        in_progress: PersistentHashSet<String>,
        result: Vec<String>,
        work_stack: Vec<WorkItem>,
        iterations: usize,
    }

    fn process_step(
        mut state: TopoState,
        graph: Arc<HashMap<String, Vec<String>>>,
    ) -> Trampoline<Either<(Vec<String>, usize), TopoState>> {
        // Pop next work item
        let Some(work) = state.work_stack.pop() else {
            // No more work - we're done
            return Trampoline::done(Either::Right(state));
        };

        state.iterations += 1;

        match work {
            WorkItem::Enter { node, path } => {
                if state.visited.contains(&node) {
                    // Already processed, continue with next work item
                    Trampoline::suspend(move || process_step(state, graph))
                } else if state.in_progress.contains(&node) {
                    // Cycle detected
                    let mut cycle_path = path;
                    cycle_path.push(node);
                    Trampoline::done(Either::Left((cycle_path, state.iterations)))
                } else {
                    // Mark as in-progress
                    let in_progress = state.in_progress.insert(node.clone());

                    // Schedule post-visit (will be processed after all deps)
                    let mut work_stack = state.work_stack;
                    work_stack.push(WorkItem::PostVisit { node: node.clone() });

                    // Schedule dependencies (in reverse order so first dep is processed first)
                    let deps = graph.get(&node).cloned().unwrap_or_default();
                    for dep in deps.into_iter().rev() {
                        let mut dep_path = path.clone();
                        dep_path.push(node.clone());
                        work_stack.push(WorkItem::Enter {
                            node: dep,
                            path: dep_path,
                        });
                    }

                    let new_state = TopoState {
                        visited: state.visited,
                        in_progress,
                        result: state.result,
                        work_stack,
                        iterations: state.iterations,
                    };
                    Trampoline::suspend(move || process_step(new_state, graph))
                }
            }
            WorkItem::PostVisit { node } => {
                // Mark as visited, remove from in_progress, add to result
                let visited = state.visited.insert(node.clone());
                // Note: We don't actually need to remove from in_progress for correctness
                // since we check visited first. This is a trade-off for PersistentHashSet
                // which doesn't have O(1) remove.
                let mut result = state.result;
                result.push(node);

                let new_state = TopoState {
                    visited,
                    in_progress: state.in_progress,
                    result,
                    work_stack: state.work_stack,
                    iterations: state.iterations,
                };
                Trampoline::suspend(move || process_step(new_state, graph))
            }
        }
    }

    // Initialize work stack with all nodes
    let initial_work: Vec<WorkItem> = graph
        .keys()
        .cloned()
        .map(|node| WorkItem::Enter {
            node,
            path: Vec::new(),
        })
        .collect();

    let initial_state = TopoState {
        visited: PersistentHashSet::new(),
        in_progress: PersistentHashSet::new(),
        result: Vec::new(),
        work_stack: initial_work,
        iterations: 0,
    };

    let trampoline = process_step(initial_state, graph);
    match trampoline.run() {
        Either::Left(cycle_info) => (Either::Left(cycle_info.0), cycle_info.1),
        Either::Right(final_state) => (Either::Right(final_state.result), final_state.iterations),
    }
}

// =============================================================================
// GET /projects/{id}/aggregate-tree - Tree statistics aggregation
// =============================================================================

/// Aggregates task tree statistics using Trampoline and Monoid.
///
/// This handler demonstrates:
/// - **`Trampoline`**: Stack-safe recursive aggregation
/// - **`Semigroup`**: Statistics combination via `combine`
///
/// # Path Parameters
///
/// - `id`: Project UUID
///
/// # Query Parameters
///
/// - `max_depth`: Maximum tree depth to aggregate (default: 10, max: 50)
///
/// # Errors
///
/// - `400 Bad Request`: Invalid project ID format or depth parameter
/// - `404 Not Found`: Project not found
#[allow(clippy::unused_async)]
pub async fn aggregate_tree(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<AggregateTreeQuery>,
) -> Result<JsonResponse<TreeAggregationResponse>, ApiErrorResponse> {
    // Validate max_depth
    if query.max_depth > 50 {
        return Err(ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("max_depth", "max_depth must be at most 50")],
        ));
    }
    // Parse project ID
    let project_id = Uuid::parse_str(&id).map_err(|_| {
        ApiErrorResponse::validation_error(
            "Validation failed",
            vec![FieldError::new("id", "Invalid project ID format")],
        )
    })?;

    // Verify project exists
    let project = state
        .project_repository
        .find_by_id(&crate::domain::ProjectId::from_uuid(project_id))
        .await
        .map_err(ApiErrorResponse::from)?
        .ok_or_else(|| ApiErrorResponse::not_found("Project not found"))?;

    // Get tasks for project (project.tasks is a PersistentHashMap<TaskId, TaskSummary>)
    let mut tasks = Vec::new();
    let mut failed_task_ids = Vec::new();

    for (task_id, _) in &project.tasks {
        match state.task_repository.find_by_id(task_id).await {
            Ok(Some(task)) => tasks.push(task),
            Ok(None) | Err(_) => failed_task_ids.push(task_id.to_string()),
        }
    }

    // Build simulated tree structure for demonstration
    // Use configurable depth from query parameter
    let simulated_tree = build_simulated_task_tree(&tasks, query.max_depth);

    // Use Trampoline for stack-safe aggregation
    let (tree_with_stats, root_stats, iterations) = aggregate_tree_trampoline(&simulated_tree);

    Ok(JsonResponse(TreeAggregationResponse {
        project_id: id,
        root_stats,
        tree: tree_with_stats,
        trampoline_iterations: iterations,
        failed_task_ids,
    }))
}

/// Simulated tree node for aggregation demonstration.
#[derive(Debug, Clone)]
struct SimulatedTaskNode {
    task_id: String,
    title: String,
    status: TaskStatus,
    estimated_duration: u64,
    children: Vec<Self>,
}

/// Builds a simulated task tree for demonstration.
fn build_simulated_task_tree(tasks: &[Task], max_depth: usize) -> Vec<SimulatedTaskNode> {
    fn build_children(
        tasks: &[Task],
        parent_index: usize,
        current_depth: usize,
        max_depth: usize,
    ) -> Vec<SimulatedTaskNode> {
        if current_depth >= max_depth || tasks.is_empty() {
            return Vec::new();
        }

        // Simulate 2 children per node at each level
        let child_count = 2.min(tasks.len().saturating_sub(1));
        (0..child_count)
            .filter_map(|i| {
                let child_index = (parent_index + 1 + i) % tasks.len();
                if child_index == parent_index {
                    return None;
                }
                let task = &tasks[child_index];
                Some(SimulatedTaskNode {
                    task_id: format!("{}-child-{i}", task.task_id),
                    title: format!("{} (level {})", task.title, current_depth),
                    status: task.status,
                    estimated_duration: 30,
                    children: build_children(tasks, child_index, current_depth + 1, max_depth),
                })
            })
            .collect()
    }

    tasks
        .iter()
        .enumerate()
        .take(3) // Limit root nodes
        .map(|(i, task)| SimulatedTaskNode {
            task_id: task.task_id.to_string(),
            title: task.title.clone(),
            status: task.status,
            estimated_duration: 60,
            children: build_children(tasks, i, 1, max_depth),
        })
        .collect()
}

/// Pure: Aggregates tree statistics using Trampoline.
///
/// This implementation uses a post-order traversal via work stack to ensure true stack safety.
/// Each node is processed only after all its children have been processed and their stats
/// are available.
#[allow(clippy::too_many_lines)]
fn aggregate_tree_trampoline(
    nodes: &[SimulatedTaskNode],
) -> (Vec<TaskNodeStats>, AggregatedStats, usize) {
    // Lightweight node info for PostVisit (avoids cloning children)
    #[derive(Clone)]
    struct NodeInfo {
        task_id: String,
        title: String,
        status: TaskStatus,
        estimated_duration: u64,
    }

    // Work item for post-order traversal
    #[derive(Clone)]
    enum WorkItem {
        // First visit: schedule children, then schedule PostVisit
        Enter {
            node: SimulatedTaskNode,
            parent_index: Option<usize>, // Index in partial_results where parent's children go
        },
        // After children are processed: aggregate and store result
        PostVisit {
            node_info: NodeInfo,
            child_count: usize,
            parent_index: Option<usize>,
        },
    }

    // Partial result during traversal
    #[derive(Clone)]
    struct PartialResult {
        stats: TaskNodeStats,
    }

    // State for aggregation
    #[derive(Clone)]
    struct AggState {
        work_stack: Vec<WorkItem>,
        // Stack of completed child results (children of current node being processed)
        result_stack: Vec<PartialResult>,
        // Final top-level results
        root_results: Vec<TaskNodeStats>,
        iterations: usize,
    }

    fn process_step(mut state: AggState) -> Trampoline<AggState> {
        let Some(work) = state.work_stack.pop() else {
            // No more work
            return Trampoline::done(state);
        };

        state.iterations += 1;

        match work {
            WorkItem::Enter { node, parent_index } => {
                if node.children.is_empty() {
                    // Leaf node: compute stats immediately and push to result stack
                    let is_completed = node.status == TaskStatus::Completed;
                    let leaf_aggregated = AggregatedStats {
                        total_tasks: 1,
                        completed_tasks: usize::from(is_completed),
                        completion_rate: if is_completed { 1.0 } else { 0.0 },
                        total_estimated_duration: node.estimated_duration,
                    };

                    let node_stats = TaskNodeStats {
                        task_id: node.task_id,
                        title: node.title,
                        stats: leaf_aggregated,
                        children: Vec::new(),
                    };

                    // If this is a root node (no parent), add to root_results
                    // Otherwise, push to result_stack for parent to collect
                    if parent_index.is_none() {
                        state.root_results.push(node_stats);
                    } else {
                        state.result_stack.push(PartialResult { stats: node_stats });
                    }

                    Trampoline::suspend(move || process_step(state))
                } else {
                    // Internal node: schedule PostVisit first, then children
                    let child_count = node.children.len();

                    // Extract only necessary info for PostVisit (avoid cloning children)
                    let node_info = NodeInfo {
                        task_id: node.task_id.clone(),
                        title: node.title.clone(),
                        status: node.status,
                        estimated_duration: node.estimated_duration,
                    };

                    // PostVisit will be processed after all children
                    state.work_stack.push(WorkItem::PostVisit {
                        node_info,
                        child_count,
                        parent_index,
                    });

                    // Schedule children (in reverse order so first child is processed first)
                    for child in node.children.into_iter().rev() {
                        state.work_stack.push(WorkItem::Enter {
                            node: child,
                            parent_index: Some(state.result_stack.len()), // Current result stack position
                        });
                    }

                    Trampoline::suspend(move || process_step(state))
                }
            }
            WorkItem::PostVisit {
                node_info,
                child_count,
                parent_index,
            } => {
                // Collect child results from result_stack
                let mut child_stats_list = Vec::with_capacity(child_count);
                let mut aggregated = AggregatedStats::default();

                // Pop child_count results from result_stack
                for _ in 0..child_count {
                    if let Some(partial) = state.result_stack.pop() {
                        aggregated = aggregated.combine(partial.stats.stats.clone());
                        child_stats_list.push(partial.stats);
                    }
                }

                // Reverse to restore original order (since we popped in reverse)
                child_stats_list.reverse();

                // Add self to aggregation
                let self_stats = AggregatedStats {
                    total_tasks: 1,
                    completed_tasks: usize::from(node_info.status == TaskStatus::Completed),
                    completion_rate: 0.0, // Will be recalculated by combine
                    total_estimated_duration: node_info.estimated_duration,
                };
                let combined = aggregated.combine(self_stats);

                let node_stats = TaskNodeStats {
                    task_id: node_info.task_id,
                    title: node_info.title,
                    stats: combined,
                    children: child_stats_list,
                };

                // If this is a root node, add to root_results
                // Otherwise, push to result_stack for parent
                if parent_index.is_none() {
                    state.root_results.push(node_stats);
                } else {
                    state.result_stack.push(PartialResult { stats: node_stats });
                }

                Trampoline::suspend(move || process_step(state))
            }
        }
    }

    // Initialize work stack with all root nodes
    let initial_work: Vec<WorkItem> = nodes
        .iter()
        .rev() // Reverse so first node is at top of stack
        .map(|node| WorkItem::Enter {
            node: node.clone(),
            parent_index: None, // Root nodes have no parent
        })
        .collect();

    let initial_state = AggState {
        work_stack: initial_work,
        result_stack: Vec::new(),
        root_results: Vec::new(),
        iterations: 0,
    };

    let trampoline = process_step(initial_state);
    let final_state = trampoline.run();

    // Aggregate root-level stats
    let mut root_aggregated = AggregatedStats::default();
    for node_stats in &final_state.root_results {
        root_aggregated = root_aggregated.combine(node_stats.stats.clone());
    }

    (
        final_state.root_results,
        root_aggregated,
        final_state.iterations,
    )
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // Flatten Subtasks Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_flatten_subtasks_empty() {
        let subtasks: Vec<SimulatedSubTask> = Vec::new();
        let (result, iterations, max_depth) = flatten_subtasks_trampoline(&subtasks, 100);

        assert!(result.is_empty());
        assert_eq!(iterations, 0); // Empty work returns immediately without incrementing
        assert_eq!(max_depth, 0);
    }

    #[rstest]
    fn test_flatten_subtasks_single_level() {
        let subtasks = vec![
            SimulatedSubTask {
                id: "1".to_string(),
                title: "Task 1".to_string(),
                children: Vec::new(),
            },
            SimulatedSubTask {
                id: "2".to_string(),
                title: "Task 2".to_string(),
                children: Vec::new(),
            },
        ];

        let (result, _, max_depth) = flatten_subtasks_trampoline(&subtasks, 100);

        assert_eq!(result.len(), 2);
        assert_eq!(max_depth, 0);
    }

    #[rstest]
    fn test_flatten_subtasks_nested() {
        let subtasks = vec![SimulatedSubTask {
            id: "root".to_string(),
            title: "Root".to_string(),
            children: vec![
                SimulatedSubTask {
                    id: "child1".to_string(),
                    title: "Child 1".to_string(),
                    children: vec![SimulatedSubTask {
                        id: "grandchild".to_string(),
                        title: "Grandchild".to_string(),
                        children: Vec::new(),
                    }],
                },
                SimulatedSubTask {
                    id: "child2".to_string(),
                    title: "Child 2".to_string(),
                    children: Vec::new(),
                },
            ],
        }];

        let (result, _, max_depth) = flatten_subtasks_trampoline(&subtasks, 100);

        assert_eq!(result.len(), 4); // root + child1 + child2 + grandchild
        assert_eq!(max_depth, 2); // grandchild is at depth 2
    }

    #[rstest]
    fn test_flatten_subtasks_respects_max_depth() {
        let subtasks = SimulatedSubTask::generate_tree(10, 2, 0, "root");
        let (result_limited, _, max_depth_limited) = flatten_subtasks_trampoline(&subtasks, 3);
        let (result_full, _, max_depth_full) = flatten_subtasks_trampoline(&subtasks, 100);

        assert!(result_limited.len() < result_full.len());
        assert!(max_depth_limited <= 3);
        assert!(max_depth_full > max_depth_limited);
    }

    #[rstest]
    fn test_flatten_subtasks_deep_hierarchy() {
        // Test stack safety with deep hierarchy
        let subtasks = SimulatedSubTask::generate_tree(15, 1, 0, "root");
        let (result, iterations, max_depth) = flatten_subtasks_trampoline(&subtasks, 100);

        assert!(!result.is_empty());
        assert!(iterations > 0);
        assert_eq!(max_depth, 14); // 0-indexed depth
    }

    // -------------------------------------------------------------------------
    // Topological Sort Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_topological_sort_empty() {
        let graph: HashMap<String, Vec<String>> = HashMap::new();
        let (result, _) = topological_sort_trampoline(&graph);

        assert!(matches!(result, Either::Right(order) if order.is_empty()));
    }

    #[rstest]
    fn test_topological_sort_single_node() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), Vec::new());

        let (result, _) = topological_sort_trampoline(&graph);

        assert!(matches!(result, Either::Right(order) if order == vec!["A"]));
    }

    #[rstest]
    fn test_topological_sort_linear_chain() {
        let mut graph = HashMap::new();
        graph.insert("C".to_string(), Vec::new());
        graph.insert("B".to_string(), vec!["C".to_string()]);
        graph.insert("A".to_string(), vec!["B".to_string()]);

        let (result, _) = topological_sort_trampoline(&graph);

        match result {
            Either::Right(order) => {
                // C should come before B, B should come before A
                let pos_a = order.iter().position(|x| x == "A").unwrap();
                let pos_b = order.iter().position(|x| x == "B").unwrap();
                let pos_c = order.iter().position(|x| x == "C").unwrap();
                assert!(pos_c < pos_b);
                assert!(pos_b < pos_a);
            }
            Either::Left(_) => panic!("Expected no cycle"),
        }
    }

    #[rstest]
    fn test_topological_sort_no_cycle() {
        let mut graph = HashMap::new();
        graph.insert("D".to_string(), Vec::new());
        graph.insert("C".to_string(), vec!["D".to_string()]);
        graph.insert("B".to_string(), vec!["D".to_string()]);
        graph.insert("A".to_string(), vec!["B".to_string(), "C".to_string()]);

        let (result, _) = topological_sort_trampoline(&graph);

        assert!(matches!(result, Either::Right(_)));
    }

    // -------------------------------------------------------------------------
    // Aggregated Stats Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_aggregated_stats_combine() {
        let stats1 = AggregatedStats {
            total_tasks: 5,
            completed_tasks: 3,
            completion_rate: 0.6,
            total_estimated_duration: 100,
        };

        let stats2 = AggregatedStats {
            total_tasks: 5,
            completed_tasks: 2,
            completion_rate: 0.4,
            total_estimated_duration: 50,
        };

        let combined = stats1.combine(stats2);

        assert_eq!(combined.total_tasks, 10);
        assert_eq!(combined.completed_tasks, 5);
        assert!((combined.completion_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(combined.total_estimated_duration, 150);
    }

    #[rstest]
    fn test_aggregated_stats_combine_empty() {
        let stats = AggregatedStats {
            total_tasks: 5,
            completed_tasks: 3,
            completion_rate: 0.6,
            total_estimated_duration: 100,
        };

        let empty = AggregatedStats::default();
        let combined = stats.clone().combine(empty);

        assert_eq!(combined.total_tasks, stats.total_tasks);
        assert_eq!(combined.completed_tasks, stats.completed_tasks);
    }

    // -------------------------------------------------------------------------
    // Tree Aggregation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_aggregate_tree_empty() {
        let nodes: Vec<SimulatedTaskNode> = Vec::new();
        let (tree, root_stats, iterations) = aggregate_tree_trampoline(&nodes);

        assert!(tree.is_empty());
        assert_eq!(root_stats.total_tasks, 0);
        assert_eq!(iterations, 0);
    }

    #[rstest]
    fn test_aggregate_tree_single_node() {
        let nodes = vec![SimulatedTaskNode {
            task_id: "1".to_string(),
            title: "Task 1".to_string(),
            status: TaskStatus::Completed,
            estimated_duration: 60,
            children: Vec::new(),
        }];

        let (tree, root_stats, _) = aggregate_tree_trampoline(&nodes);

        assert_eq!(tree.len(), 1);
        assert_eq!(root_stats.total_tasks, 1);
        assert_eq!(root_stats.completed_tasks, 1);
    }

    #[rstest]
    fn test_aggregate_tree_with_children() {
        let nodes = vec![SimulatedTaskNode {
            task_id: "parent".to_string(),
            title: "Parent".to_string(),
            status: TaskStatus::InProgress,
            estimated_duration: 60,
            children: vec![
                SimulatedTaskNode {
                    task_id: "child1".to_string(),
                    title: "Child 1".to_string(),
                    status: TaskStatus::Completed,
                    estimated_duration: 30,
                    children: Vec::new(),
                },
                SimulatedTaskNode {
                    task_id: "child2".to_string(),
                    title: "Child 2".to_string(),
                    status: TaskStatus::Pending,
                    estimated_duration: 30,
                    children: Vec::new(),
                },
            ],
        }];

        let (tree, root_stats, _) = aggregate_tree_trampoline(&nodes);

        assert_eq!(tree.len(), 1);
        assert_eq!(root_stats.total_tasks, 3); // parent + 2 children
        assert_eq!(root_stats.completed_tasks, 1); // only child1
        assert_eq!(root_stats.total_estimated_duration, 120); // 60 + 30 + 30
    }
}
