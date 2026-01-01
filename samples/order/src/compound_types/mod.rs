//! 注文ドメインで使用する複合型（Compound Types）
//!
//! Phase 1 で定義した基本型を組み合わせて、より高レベルなドメインエンティティを表現する。
//! lambars の Optics（Lens）を活用して不変データの更新を効率的に行う。
//!
//! # 型一覧
//!
//! - [`PersonalName`] - 個人名（姓・名）
//! - [`CustomerInfo`] - 顧客情報（個人名、メール、VIP ステータス）
//! - [`Address`] - 住所
//!
//! # Lens の使用例
//!
//! ```
//! use order_taking_sample::compound_types::{PersonalName, CustomerInfo};
//! use order_taking_sample::simple_types::String50;
//! use lambars::optics::Lens;
//!
//! // CustomerInfo から first_name を取得（Lens 合成）
//! let customer = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
//!
//! let name_lens = CustomerInfo::name_lens();
//! let first_name_lens = PersonalName::first_name_lens();
//! let customer_first_name = name_lens.compose(first_name_lens);
//!
//! let first_name = customer_first_name.get(&customer);
//! assert_eq!(first_name.value(), "John");
//! ```

mod address;
mod customer_info;
mod personal_name;

pub use address::Address;
pub use customer_info::CustomerInfo;
pub use personal_name::PersonalName;
