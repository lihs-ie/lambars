//! 出力 DTO
//!
//! API レスポンスのシリアライズに使用する DTO 型を定義する。
//! 後続のステップで実装する。

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::dto::AddressDto;
use crate::workflow::{
    BillableOrderPlaced, OrderAcknowledgmentSent, PlaceOrderEvent, PricedOrderLine,
    PricedOrderProductLine, ShippableOrderLine, ShippableOrderPlaced,
};

// =============================================================================
// ShippableOrderLineDto (REQ-079)
// =============================================================================

/// 配送対象の注文明細 DTO
///
/// 配送イベント内の明細情報をシリアライズするための型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShippableOrderLineDto {
    /// 製品コード
    pub product_code: String,
    /// 数量（文字列形式）
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
}

impl ShippableOrderLineDto {
    /// ドメインの `ShippableOrderLine` から `ShippableOrderLineDto` を生成する
    #[must_use]
    pub fn from_domain(line: &ShippableOrderLine) -> Self {
        Self {
            product_code: line.product_code().value().to_string(),
            quantity: line.quantity().value(),
        }
    }
}

// =============================================================================
// ShippableOrderPlacedDto (REQ-080)
// =============================================================================

/// 配送可能注文確定イベント DTO
///
/// 配送イベントをシリアライズするための型。
/// PDF データは Base64 エンコードされる。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShippableOrderPlacedDto {
    /// 注文ID
    pub order_id: String,
    /// 配送先住所
    pub shipping_address: AddressDto,
    /// 配送明細リスト
    pub shipment_lines: Vec<ShippableOrderLineDto>,
    /// PDF ファイル名
    pub pdf_name: String,
    /// PDF データ（Base64 エンコード）
    pub pdf_data: String,
}

impl ShippableOrderPlacedDto {
    /// ドメインの `ShippableOrderPlaced` から `ShippableOrderPlacedDto` を生成する
    #[must_use]
    pub fn from_domain(event: &ShippableOrderPlaced) -> Self {
        use base64::Engine;
        let pdf_data = base64::engine::general_purpose::STANDARD.encode(event.pdf().bytes());

        Self {
            order_id: event.order_id().value().to_string(),
            shipping_address: AddressDto::from_address(event.shipping_address()),
            shipment_lines: event
                .shipment_lines()
                .iter()
                .map(ShippableOrderLineDto::from_domain)
                .collect(),
            pdf_name: event.pdf().name().to_string(),
            pdf_data,
        }
    }
}

// =============================================================================
// BillableOrderPlacedDto (REQ-081)
// =============================================================================

/// 請求可能注文確定イベント DTO
///
/// 請求イベントをシリアライズするための型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillableOrderPlacedDto {
    /// 注文ID
    pub order_id: String,
    /// 請求先住所
    pub billing_address: AddressDto,
    /// 請求金額（文字列形式）
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_to_bill: Decimal,
}

impl BillableOrderPlacedDto {
    /// ドメインの `BillableOrderPlaced` から `BillableOrderPlacedDto` を生成する
    #[must_use]
    pub fn from_domain(event: &BillableOrderPlaced) -> Self {
        Self {
            order_id: event.order_id().value().to_string(),
            billing_address: AddressDto::from_address(event.billing_address()),
            amount_to_bill: event.amount_to_bill().value(),
        }
    }
}

// =============================================================================
// OrderAcknowledgmentSentDto (REQ-082)
// =============================================================================

/// 注文確認メール送信イベント DTO
///
/// 確認メール送信イベントをシリアライズするための型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderAcknowledgmentSentDto {
    /// 注文ID
    pub order_id: String,
    /// 送信先メールアドレス
    pub email_address: String,
}

impl OrderAcknowledgmentSentDto {
    /// ドメインの `OrderAcknowledgmentSent` から `OrderAcknowledgmentSentDto` を生成する
    #[must_use]
    pub fn from_domain(event: &OrderAcknowledgmentSent) -> Self {
        Self {
            order_id: event.order_id().value().to_string(),
            email_address: event.email_address().value().to_string(),
        }
    }
}

// =============================================================================
// PricedOrderProductLineDto (REQ-083)
// =============================================================================

/// 価格付き製品注文明細 DTO
///
/// 価格付き製品明細をシリアライズするための型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricedOrderProductLineDto {
    /// 注文明細ID
    pub order_line_id: String,
    /// 製品コード
    pub product_code: String,
    /// 数量（文字列形式）
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// 明細価格（文字列形式）
    #[serde(with = "rust_decimal::serde::str")]
    pub line_price: Decimal,
}

impl PricedOrderProductLineDto {
    /// ドメインの `PricedOrderProductLine` から `PricedOrderProductLineDto` を生成する
    #[must_use]
    pub fn from_domain(line: &PricedOrderProductLine) -> Self {
        Self {
            order_line_id: line.order_line_id().value().to_string(),
            product_code: line.product_code().value().to_string(),
            quantity: line.quantity().value(),
            line_price: line.line_price().value(),
        }
    }
}

// =============================================================================
// PricedOrderLineDto (REQ-084)
// =============================================================================

/// 価格付き注文明細 DTO
///
/// 価格付き明細（製品またはコメント）をシリアライズするための型。
/// `type` フィールドで判別する隣接タグ形式。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PricedOrderLineDto {
    /// 製品明細
    ProductLine(PricedOrderProductLineDto),
    /// コメント行
    CommentLine(String),
}

impl PricedOrderLineDto {
    /// ドメインの `PricedOrderLine` から `PricedOrderLineDto` を生成する
    #[must_use]
    pub fn from_domain(line: &PricedOrderLine) -> Self {
        match line {
            PricedOrderLine::ProductLine(product_line) => {
                Self::ProductLine(PricedOrderProductLineDto::from_domain(product_line))
            }
            PricedOrderLine::CommentLine(comment) => Self::CommentLine(comment.clone()),
        }
    }
}

// =============================================================================
// PlaceOrderEventDto (REQ-085)
// =============================================================================

/// `PlaceOrder` ワークフローの出力イベント DTO
///
/// ワークフロー完了時のイベントをシリアライズするための型。
/// `type` フィールドで判別する隣接タグ形式。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PlaceOrderEventDto {
    /// 配送可能注文確定イベント
    ShippableOrderPlaced(ShippableOrderPlacedDto),
    /// 請求可能注文確定イベント
    BillableOrderPlaced(BillableOrderPlacedDto),
    /// 確認メール送信イベント
    AcknowledgmentSent(OrderAcknowledgmentSentDto),
}

impl PlaceOrderEventDto {
    /// ドメインの `PlaceOrderEvent` から `PlaceOrderEventDto` を生成する
    #[must_use]
    pub fn from_domain(event: &PlaceOrderEvent) -> Self {
        match event {
            PlaceOrderEvent::ShippableOrderPlaced(e) => {
                Self::ShippableOrderPlaced(ShippableOrderPlacedDto::from_domain(e))
            }
            PlaceOrderEvent::BillableOrderPlaced(e) => {
                Self::BillableOrderPlaced(BillableOrderPlacedDto::from_domain(e))
            }
            PlaceOrderEvent::AcknowledgmentSent(e) => {
                Self::AcknowledgmentSent(OrderAcknowledgmentSentDto::from_domain(e))
            }
        }
    }

    /// ドメインイベントのリストから DTO リストを生成する
    #[must_use]
    pub fn from_domain_list(events: &[PlaceOrderEvent]) -> Vec<Self> {
        events.iter().map(Self::from_domain).collect()
    }
}
