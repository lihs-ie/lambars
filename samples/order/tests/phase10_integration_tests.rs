//! Phase 10 統合テスト
//!
//! PricingCatalog と acknowledge_order_with_logging を使用した統合テスト。

use functional_rusty::effect::IO;
use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{BillingAmount, OrderId, Price, ProductCode};
use order_taking_sample::workflow::{
    HtmlString, OrderAcknowledgment, PricedOrder, PricedOrderWithShippingMethod, PricingCatalog,
    PricingMethod, SendResult, acknowledge_order_with_logging, add_shipping_info_to_order,
    calculate_shipping_cost, create_catalog_pricing_function,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;

// =============================================================================
// テストヘルパー
// =============================================================================

/// テスト用の ProductCode を作成する
fn create_test_product_code(code: &str) -> ProductCode {
    ProductCode::create("field", code).unwrap()
}

/// テスト用の Price を作成する
fn create_test_price(value: u32) -> Price {
    Price::create(Decimal::from(value)).unwrap()
}

/// テスト用の PricedOrder を作成する
fn create_test_priced_order(amount: Decimal) -> PricedOrder {
    let order_id = OrderId::create("OrderId", "order-001").unwrap();
    let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
    let address = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "US")
        .expect("Valid address");
    let billing_amount = BillingAmount::create(amount).unwrap();

    PricedOrder::new(
        order_id,
        customer_info,
        address.clone(),
        address,
        billing_amount,
        vec![],
        PricingMethod::Standard,
    )
}

// =============================================================================
// PricingCatalog 統合テスト
// =============================================================================

#[rstest]
fn test_catalog_pricing_function_found() {
    let widget_code = create_test_product_code("W1234");
    let gizmo_code = create_test_product_code("G123");
    let widget_price = create_test_price(100);
    let gizmo_price = create_test_price(50);
    let default_price = create_test_price(10);

    let catalog = PricingCatalog::new()
        .set_price(&widget_code, widget_price)
        .set_price(&gizmo_code, gizmo_price);

    let get_price = create_catalog_pricing_function(catalog, default_price);

    // カタログにある商品は設定した価格を返す
    assert_eq!(get_price(&widget_code).value(), Decimal::from(100));
    assert_eq!(get_price(&gizmo_code).value(), Decimal::from(50));
}

#[rstest]
fn test_catalog_pricing_function_not_found() {
    let widget_code = create_test_product_code("W1234");
    let unknown_code = create_test_product_code("W9999");
    let default_price = create_test_price(25);

    let catalog = PricingCatalog::singleton(&widget_code, create_test_price(100));
    let get_price = create_catalog_pricing_function(catalog, default_price);

    // カタログにない商品はデフォルト価格を返す
    assert_eq!(get_price(&unknown_code).value(), Decimal::from(25));
}

#[rstest]
fn test_catalog_merge_workflow() {
    // ベースカタログと拡張カタログをマージ
    let base_catalog = PricingCatalog::new()
        .set_price(&create_test_product_code("W1234"), create_test_price(100))
        .set_price(&create_test_product_code("G123"), create_test_price(50));

    let extension_catalog = PricingCatalog::new()
        .set_price(&create_test_product_code("W5678"), create_test_price(150))
        .set_price(&create_test_product_code("W1234"), create_test_price(120)); // 上書き

    let merged_catalog = base_catalog.merge(&extension_catalog);

    // 検証
    assert_eq!(merged_catalog.len(), 3);

    // W1234 は extension の値（120）で上書き
    assert_eq!(
        merged_catalog
            .get_price(&create_test_product_code("W1234"))
            .unwrap()
            .value(),
        Decimal::from(120)
    );

    // G123 は元のまま
    assert_eq!(
        merged_catalog
            .get_price(&create_test_product_code("G123"))
            .unwrap()
            .value(),
        Decimal::from(50)
    );

    // W5678 は extension から追加
    assert_eq!(
        merged_catalog
            .get_price(&create_test_product_code("W5678"))
            .unwrap()
            .value(),
        Decimal::from(150)
    );
}

#[rstest]
fn test_catalog_immutability_in_workflow() {
    // 元のカタログ
    let original_catalog =
        PricingCatalog::singleton(&create_test_product_code("W1234"), create_test_price(100));

    // 価格取得関数を作成
    let get_price1 =
        create_catalog_pricing_function(original_catalog.clone(), create_test_price(10));

    // カタログを「更新」（実際は新しいカタログを作成）
    let updated_catalog =
        original_catalog.set_price(&create_test_product_code("W1234"), create_test_price(200));

    let get_price2 = create_catalog_pricing_function(updated_catalog, create_test_price(10));

    // 元のカタログを使った価格取得関数は元の価格を返す
    assert_eq!(
        get_price1(&create_test_product_code("W1234")).value(),
        Decimal::from(100)
    );

    // 更新後のカタログを使った価格取得関数は新しい価格を返す
    assert_eq!(
        get_price2(&create_test_product_code("W1234")).value(),
        Decimal::from(200)
    );
}

// =============================================================================
// acknowledge_order_with_logging 統合テスト
// =============================================================================

#[rstest]
fn test_acknowledge_order_with_logging_integration() {
    // テスト用の注文を作成
    let priced_order = create_test_priced_order(Decimal::from(100));

    // 配送情報を追加
    let order_with_shipping = add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

    // ログ記録用
    let log_messages: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let log_messages_clone = log_messages.clone();

    let log_action = move |message: &str| {
        let messages = log_messages_clone.clone();
        let message = message.to_string();
        IO::new(move || {
            messages.borrow_mut().push(message.clone());
        })
    };

    let create_letter = |order: &PricedOrderWithShippingMethod| {
        let content = format!(
            "<p>Order {} confirmed. Total: ${}</p>",
            order.priced_order().order_id().value(),
            order.priced_order().amount_to_bill().value()
        );
        HtmlString::new(content)
    };

    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

    // 確認メール送信を実行
    let io_result = acknowledge_order_with_logging(
        &create_letter,
        &send_acknowledgment,
        &log_action,
        &order_with_shipping,
    );

    let result = io_result.run_unsafe();

    // 送信成功を検証
    assert!(result.is_some());
    let event = result.unwrap();
    assert_eq!(event.order_id().value(), "order-001");
    assert_eq!(event.email_address().value(), "john@example.com");

    // ログ出力を検証
    let messages = log_messages.borrow();
    assert_eq!(messages.len(), 3);
    assert!(messages[0].contains("Creating"));
    assert!(messages[1].contains("Sending"));
    assert!(messages[2].contains("completed"));
}

#[rstest]
fn test_acknowledge_order_with_logging_not_sent() {
    let priced_order = create_test_priced_order(Decimal::from(100));
    let order_with_shipping = add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

    let log_messages: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let log_messages_clone = log_messages.clone();

    let log_action = move |message: &str| {
        let messages = log_messages_clone.clone();
        let message = message.to_string();
        IO::new(move || {
            messages.borrow_mut().push(message.clone());
        })
    };

    let create_letter =
        |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Test</p>".to_string());

    // 送信失敗をシミュレート
    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::NotSent);

    let io_result = acknowledge_order_with_logging(
        &create_letter,
        &send_acknowledgment,
        &log_action,
        &order_with_shipping,
    );

    let result = io_result.run_unsafe();

    // 送信失敗時は None
    assert!(result.is_none());

    // ログは全て出力される
    let messages = log_messages.borrow();
    assert_eq!(messages.len(), 3);
}

// =============================================================================
// 完全なワークフロー統合テスト
// =============================================================================

#[rstest]
fn test_complete_workflow_with_catalog_and_logging() {
    // 1. カタログ作成
    let catalog = PricingCatalog::new()
        .set_price(&create_test_product_code("W1234"), create_test_price(100))
        .set_price(&create_test_product_code("G123"), create_test_price(50));

    // 2. 価格取得関数生成
    let get_price = create_catalog_pricing_function(catalog, create_test_price(10));

    // 3. 価格を確認
    assert_eq!(
        get_price(&create_test_product_code("W1234")).value(),
        Decimal::from(100)
    );

    // 4. 仮の注文データ（すでに価格計算済みとして）
    let priced_order = create_test_priced_order(Decimal::from(250));

    // 5. 配送情報追加
    let order_with_shipping = add_shipping_info_to_order(&calculate_shipping_cost, &priced_order);

    // NY は遠方州なので $10
    assert_eq!(
        order_with_shipping.shipping_info().shipping_cost().value(),
        Decimal::from(10)
    );

    // 6. 確認メール送信（eff! マクロ使用）
    let log_messages: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let log_messages_clone = log_messages.clone();

    let log_action = move |message: &str| {
        let messages = log_messages_clone.clone();
        let message = message.to_string();
        IO::new(move || {
            messages.borrow_mut().push(message.clone());
        })
    };

    let create_letter =
        |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Confirmed</p>".to_string());

    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

    let io_result = acknowledge_order_with_logging(
        &create_letter,
        &send_acknowledgment,
        &log_action,
        &order_with_shipping,
    );

    // 7. イベント生成
    let result = io_result.run_unsafe();
    assert!(result.is_some());

    let event = result.unwrap();
    assert_eq!(event.order_id().value(), "order-001");
    assert_eq!(event.email_address().value(), "john@example.com");

    // ログが正しく出力されていることを確認
    let messages = log_messages.borrow();
    assert_eq!(messages.len(), 3);
}
