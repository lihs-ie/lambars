//! # Order Taking Sample Application
//!
//! lambars を使用した注文処理サンプルアプリケーション。
//!
//! ## 概要
//!
//! F# の「Domain Modeling Made Functional」を参考に、
//! Rust で関数型ドメインモデリングを実現するサンプルです。
//!
//! ## モジュール構成
//!
//! - `simple_types`: 制約付き基本型（`String50`, `EmailAddress`, `OrderId` 等）
//! - `compound_types`: 複合型（`PersonalName`, `CustomerInfo`, `Address` 等）
//! - `workflow`: ワークフロー型定義（状態遷移を型で表現）

#![forbid(unsafe_code)]

pub mod api;
pub mod compound_types;
pub mod dto;
pub mod simple_types;
pub mod workflow;
