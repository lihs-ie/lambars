//! Phase 11: pipe! マクロ統合テスト
//!
//! pipe! マクロを使用した関数の動作を検証する。
//! 既存のテストは events_tests.rs にあるため、
//! ここでは pipe! マクロ固有のパターンを検証する。

use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{
    BillingAmount, EmailAddress, OrderId, OrderLineId, OrderQuantity, Price, ProductCode,
};
use order_taking_sample::workflow::{
    OrderAcknowledgmentSent, PricedOrder, PricedOrderLine, PricedOrderProductLine, PricingMethod,
    create_billing_event, create_events, create_shipping_event,
};
use rstest::rstest;
use rust_decimal::Decimal;

// =============================================================================
// テストヘルパー
// =============================================================================

fn create_test_priced_order(
    order_id: &str,
    amount_to_bill: Decimal,
    lines: Vec<PricedOrderLine>,
) -> PricedOrder {
    let order_id = OrderId::create("OrderId", order_id).unwrap();
    let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
    let address =
        Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "US").unwrap();
    let amount = BillingAmount::create(amount_to_bill).unwrap();

    PricedOrder::new(
        order_id,
        customer_info,
        address.clone(),
        address,
        amount,
        lines,
        PricingMethod::Standard,
    )
}

fn create_test_product_line(line_id: &str, product_code: &str, price: Decimal) -> PricedOrderLine {
    let order_line_id = OrderLineId::create("OrderLineId", line_id).unwrap();
    let product_code = ProductCode::create("ProductCode", product_code).unwrap();
    let quantity = OrderQuantity::create("Quantity", &product_code, Decimal::from(5)).unwrap();
    let line_price = Price::create(price).unwrap();

    PricedOrderLine::ProductLine(PricedOrderProductLine::new(
        order_line_id,
        product_code,
        quantity,
        line_price,
    ))
}

// =============================================================================
// create_events のテスト（REQ-111）
// =============================================================================

#[rstest]
fn test_create_events_with_pipe_macro_generates_correct_order() {
    // pipe! マクロを使用した実装が正しいイベント順序を生成することを確認
    let priced_order = create_test_priced_order("order-001", Decimal::from(1000), vec![]);
    let order_id = OrderId::create("OrderId", "order-001").unwrap();
    let email = EmailAddress::create("EmailAddress", "test@example.com").unwrap();
    let acknowledgment = OrderAcknowledgmentSent::new(order_id, email);

    let events = create_events(&priced_order, Some(acknowledgment));

    // イベント順序の検証: Acknowledgment -> Shippable -> Billable
    assert_eq!(events.len(), 3);
    assert!(events[0].is_acknowledgment());
    assert!(events[1].is_shippable());
    assert!(events[2].is_billable());
}

#[rstest]
fn test_create_events_with_pipe_macro_handles_none_acknowledgment() {
    // acknowledgment が None の場合のテスト
    let priced_order = create_test_priced_order("order-002", Decimal::from(500), vec![]);

    let events = create_events(&priced_order, None);

    // Acknowledgment なし: Shippable -> Billable
    assert_eq!(events.len(), 2);
    assert!(events[0].is_shippable());
    assert!(events[1].is_billable());
}

#[rstest]
fn test_create_events_with_pipe_macro_handles_zero_billing() {
    // 請求金額が 0 の場合のテスト
    let priced_order = create_test_priced_order("order-003", Decimal::ZERO, vec![]);
    let order_id = OrderId::create("OrderId", "order-003").unwrap();
    let email = EmailAddress::create("EmailAddress", "test@example.com").unwrap();
    let acknowledgment = OrderAcknowledgmentSent::new(order_id, email);

    let events = create_events(&priced_order, Some(acknowledgment));

    // Billable なし: Acknowledgment -> Shippable
    assert_eq!(events.len(), 2);
    assert!(events[0].is_acknowledgment());
    assert!(events[1].is_shippable());
}

// =============================================================================
// create_shipping_event のテスト（REQ-112）
// =============================================================================

#[rstest]
fn test_create_shipping_event_with_pipe_macro_generates_pdf_name() {
    // pipe! マクロを使用した PDF 名生成の検証
    let priced_order = create_test_priced_order("order-001", Decimal::from(100), vec![]);

    let event = create_shipping_event(&priced_order);

    // PDF 名が正しく生成されていることを確認
    assert_eq!(event.pdf().name(), "Orderorder-001.pdf");
}

#[rstest]
fn test_create_shipping_event_with_pipe_macro_includes_order_id() {
    // pipe! マクロを使用したイベント生成で order_id が正しく設定されることを確認
    let priced_order = create_test_priced_order("order-xyz", Decimal::from(100), vec![]);

    let event = create_shipping_event(&priced_order);

    assert_eq!(event.order_id().value(), "order-xyz");
}

#[rstest]
fn test_create_shipping_event_with_pipe_macro_filters_comment_lines() {
    // pipe! マクロを使用した実装が CommentLine を正しくフィルタすることを確認
    let lines = vec![
        create_test_product_line("line-001", "W1234", Decimal::from(100)),
        PricedOrderLine::CommentLine("Gift wrapping".to_string()),
        create_test_product_line("line-002", "G567", Decimal::from(200)),
    ];
    let priced_order = create_test_priced_order("order-001", Decimal::from(300), lines);

    let event = create_shipping_event(&priced_order);

    // CommentLine はフィルタされ、ProductLine のみが含まれる
    assert_eq!(event.shipment_lines().len(), 2);
}

// =============================================================================
// create_billing_event のテスト（REQ-113）
// =============================================================================

#[rstest]
fn test_create_billing_event_with_pipe_macro_returns_some_for_positive_amount() {
    // pipe! マクロを使用した実装が正の金額で Some を返すことを確認
    let priced_order = create_test_priced_order("order-001", Decimal::from(1000), vec![]);

    let result = create_billing_event(&priced_order);

    assert!(result.is_some());
    let event = result.unwrap();
    assert_eq!(event.amount_to_bill().value(), Decimal::from(1000));
}

#[rstest]
fn test_create_billing_event_with_pipe_macro_returns_none_for_zero_amount() {
    // pipe! マクロを使用した実装が 0 金額で None を返すことを確認
    let priced_order = create_test_priced_order("order-002", Decimal::ZERO, vec![]);

    let result = create_billing_event(&priced_order);

    assert!(result.is_none());
}

#[rstest]
fn test_create_billing_event_with_pipe_macro_includes_correct_order_id() {
    // pipe! マクロを使用した実装が正しい order_id を含むことを確認
    let priced_order = create_test_priced_order("order-billing", Decimal::from(500), vec![]);

    let result = create_billing_event(&priced_order);

    assert!(result.is_some());
    let event = result.unwrap();
    assert_eq!(event.order_id().value(), "order-billing");
}

// =============================================================================
// pipe! マクロ合成テスト（REQ-117）
// =============================================================================

#[rstest]
fn test_pipe_macro_composition_in_event_pipeline() {
    // pipe! マクロによる関数合成が正しく動作することを検証
    // shipping_event -> PlaceOrderEvent::ShippableOrderPlaced の変換
    let lines = vec![create_test_product_line(
        "line-001",
        "W1234",
        Decimal::from(100),
    )];
    let priced_order = create_test_priced_order("order-composite", Decimal::from(100), lines);

    let events = create_events(&priced_order, None);

    // Shippable イベントが正しく生成されていることを確認
    assert!(events.len() >= 1);
    let shippable_event = &events[0];
    assert!(shippable_event.is_shippable());

    // ShippableOrderPlaced の内容を検証
    if let order_taking_sample::workflow::PlaceOrderEvent::ShippableOrderPlaced(event) =
        shippable_event
    {
        assert_eq!(event.order_id().value(), "order-composite");
        assert_eq!(event.shipment_lines().len(), 1);
    } else {
        panic!("Expected ShippableOrderPlaced event");
    }
}

#[rstest]
fn test_pipe_macro_preserves_data_integrity() {
    // pipe! マクロを使用してもデータの整合性が保たれることを確認
    let order_id_str = "order-integrity-test";
    let billing_amount = Decimal::new(12345, 2); // 123.45
    let lines = vec![create_test_product_line(
        "line-001",
        "W1234",
        Decimal::from(100),
    )];
    let priced_order = create_test_priced_order(order_id_str, billing_amount, lines);
    let email = EmailAddress::create("EmailAddress", "integrity@example.com").unwrap();
    let acknowledgment =
        OrderAcknowledgmentSent::new(OrderId::create("OrderId", order_id_str).unwrap(), email);

    let events = create_events(&priced_order, Some(acknowledgment));

    // 全イベントのデータ整合性を検証
    assert_eq!(events.len(), 3);

    // Acknowledgment イベント
    if let order_taking_sample::workflow::PlaceOrderEvent::AcknowledgmentSent(ack) = &events[0] {
        assert_eq!(ack.order_id().value(), order_id_str);
    }

    // Shippable イベント
    if let order_taking_sample::workflow::PlaceOrderEvent::ShippableOrderPlaced(ship) = &events[1] {
        assert_eq!(ship.order_id().value(), order_id_str);
        assert_eq!(ship.pdf().name(), format!("Order{}.pdf", order_id_str));
    }

    // Billable イベント
    if let order_taking_sample::workflow::PlaceOrderEvent::BillableOrderPlaced(bill) = &events[2] {
        assert_eq!(bill.order_id().value(), order_id_str);
        assert_eq!(bill.amount_to_bill().value(), billing_amount);
    }
}
