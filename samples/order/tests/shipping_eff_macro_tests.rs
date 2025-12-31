//! eff! マクロ使用テスト
//!
//! acknowledge_order_with_logging 関数のテストを実装する。
//! eff! マクロの動作と IO 操作のチェーンを検証する。

use functional_rusty::effect::IO;
use order_taking_sample::compound_types::{Address, CustomerInfo};
use order_taking_sample::simple_types::{BillingAmount, OrderId, Price};
use order_taking_sample::workflow::{
    HtmlString, OrderAcknowledgment, PricedOrder, PricedOrderWithShippingMethod, PricingMethod,
    SendResult, ShippingInfo, ShippingMethod, acknowledge_order_with_logging,
};
use rstest::rstest;
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// =============================================================================
// テストヘルパー
// =============================================================================

/// テスト用の配送情報付き注文を作成する
fn create_test_order_with_shipping() -> PricedOrderWithShippingMethod {
    let order_id = OrderId::create("OrderId", "order-001").unwrap();
    let customer_info = CustomerInfo::create("John", "Doe", "john@example.com", "Normal").unwrap();
    let address = Address::create("123 Main St", "", "", "", "New York", "10001", "NY", "US")
        .expect("Valid address");
    let amount_to_bill = BillingAmount::create(Decimal::from(100)).unwrap();

    let priced_order = PricedOrder::new(
        order_id,
        customer_info,
        address.clone(),
        address,
        amount_to_bill,
        vec![],
        PricingMethod::Standard,
    );

    let shipping_info = ShippingInfo::new(
        ShippingMethod::Fedex24,
        Price::create(Decimal::from(10)).unwrap(),
    );

    PricedOrderWithShippingMethod::new(shipping_info, priced_order)
}

// =============================================================================
// acknowledge_order_with_logging テスト
// =============================================================================

#[rstest]
fn test_acknowledge_order_with_logging_sent() {
    let order = create_test_order_with_shipping();
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

    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

    let io_result =
        acknowledge_order_with_logging(&create_letter, &send_acknowledgment, &log_action, &order);

    let result = io_result.run_unsafe();

    assert!(result.is_some());
    let event = result.unwrap();
    assert_eq!(event.order_id().value(), "order-001");
    assert_eq!(event.email_address().value(), "john@example.com");

    // ログ順序を検証
    let messages = log_messages.borrow();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0], "Creating acknowledgment letter");
    assert_eq!(messages[1], "Sending acknowledgment email");
    assert_eq!(messages[2], "Acknowledgment process completed");
}

#[rstest]
fn test_acknowledge_order_with_logging_not_sent() {
    let order = create_test_order_with_shipping();
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

    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::NotSent);

    let io_result =
        acknowledge_order_with_logging(&create_letter, &send_acknowledgment, &log_action, &order);

    let result = io_result.run_unsafe();

    // 送信失敗時は None
    assert!(result.is_none());

    // ログは全て出力される
    let messages = log_messages.borrow();
    assert_eq!(messages.len(), 3);
}

#[rstest]
fn test_acknowledge_order_with_logging_deferred_execution() {
    let order = create_test_order_with_shipping();
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    let log_action = move |_: &str| {
        let flag = executed_clone.clone();
        IO::new(move || {
            flag.store(true, Ordering::SeqCst);
        })
    };

    let create_letter =
        |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Test</p>".to_string());

    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

    let io_result =
        acknowledge_order_with_logging(&create_letter, &send_acknowledgment, &log_action, &order);

    // IO が生成されただけでは実行されない
    assert!(!executed.load(Ordering::SeqCst));

    // run_unsafe() で実行される
    let _ = io_result.run_unsafe();
    assert!(executed.load(Ordering::SeqCst));
}

#[rstest]
fn test_acknowledge_order_with_logging_log_order() {
    let order = create_test_order_with_shipping();
    let log_order: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));

    let log_order_clone = log_order.clone();
    let log_action = move |message: &str| {
        let order = log_order_clone.clone();
        let index = if message.contains("Creating") {
            1
        } else if message.contains("Sending") {
            2
        } else {
            3
        };
        IO::new(move || {
            order.borrow_mut().push(index);
        })
    };

    let create_letter =
        |_: &PricedOrderWithShippingMethod| HtmlString::new("<p>Test</p>".to_string());

    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

    let io_result =
        acknowledge_order_with_logging(&create_letter, &send_acknowledgment, &log_action, &order);

    let _ = io_result.run_unsafe();

    // ログが正しい順序で出力されていることを確認
    let order_vec = log_order.borrow();
    assert_eq!(*order_vec, vec![1, 2, 3]);
}

#[rstest]
fn test_acknowledge_order_with_logging_custom_letter() {
    let order = create_test_order_with_shipping();
    let letter_content: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let letter_content_clone = letter_content.clone();
    let create_letter = move |order: &PricedOrderWithShippingMethod| {
        let content = format!(
            "<p>Order {} confirmed for {}</p>",
            order.priced_order().order_id().value(),
            order
                .priced_order()
                .customer_info()
                .name()
                .first_name()
                .value()
        );
        *letter_content_clone.borrow_mut() = Some(content.clone());
        HtmlString::new(content)
    };

    let log_action = |_: &str| IO::pure(());

    let send_acknowledgment = |_: &OrderAcknowledgment| IO::pure(SendResult::Sent);

    let io_result =
        acknowledge_order_with_logging(&create_letter, &send_acknowledgment, &log_action, &order);

    let _ = io_result.run_unsafe();

    // 確認メール本文が正しく生成されていることを確認
    let content = letter_content.borrow();
    assert!(content.is_some());
    assert!(content.as_ref().unwrap().contains("order-001"));
    assert!(content.as_ref().unwrap().contains("John"));
}
