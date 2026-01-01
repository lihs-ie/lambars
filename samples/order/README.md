# サンプルアプリケーション 商品注文

# 概要

- lambars を使ったサンプルアプリケーション
- 商品の注文コマンドを受け取り実際に購入までの API

# テスト

## ローカルサーバーで起動

```shell
cd samples/order
cargo run
```

## API サンプル

### 基本的な注文（Widget 製品）

```shell
  curl -X POST http://localhost:8080/place-order \
    -H "Content-Type: application/json" \
    -d '{
      "order_id": "order-001",
      "customer_info": {
        "first_name": "John",
        "last_name": "Doe",
        "email_address": "john@example.com",
        "vip_status": "Normal"
      },
      "shipping_address": {
        "address_line1": "123 Main St",
        "address_line2": "",
        "address_line3": "",
        "address_line4": "",
        "city": "New York",
        "zip_code": "10001",
        "state": "NY",
        "country": "USA"
      },
      "billing_address": {
        "address_line1": "123 Main St",
        "address_line2": "",
        "address_line3": "",
        "address_line4": "",
        "city": "New York",
        "zip_code": "10001",
        "state": "NY",
        "country": "USA"
      },
      "lines": [
        {
          "order_line_id": "line-001",
          "product_code": "W1234",
          "quantity": "10"
        }
      ],
      "promotion_code": ""
    }'
```

### VIP 顧客 + Gizmo 製品

```shell
  curl -X POST http://localhost:8080/place-order \
    -H "Content-Type: application/json" \
    -d '{
      "order_id": "order-002",
      "customer_info": {
        "first_name": "Jane",
        "last_name": "Smith",
        "email_address": "jane@example.com",
        "vip_status": "VIP"
      },
      "shipping_address": {
        "address_line1": "456 Oak Ave",
        "address_line2": "Apt 5B",
        "address_line3": "",
        "address_line4": "",
        "city": "Los Angeles",
        "zip_code": "90001",
        "state": "CA",
        "country": "USA"
      },
      "billing_address": {
        "address_line1": "456 Oak Ave",
        "address_line2": "Apt 5B",
        "address_line3": "",
        "address_line4": "",
        "city": "Los Angeles",
        "zip_code": "90001",
        "state": "CA",
        "country": "USA"
      },
      "lines": [
        {
          "order_line_id": "line-001",
          "product_code": "G123",
          "quantity": "3.5"
        }
      ],
      "promotion_code": ""
    }'
```

### 複数製品の混合注文

```shell
  curl -X POST http://localhost:8080/place-order \
    -H "Content-Type: application/json" \
    -d '{
      "order_id": "order-003",
      "customer_info": {
        "first_name": "Bob",
        "last_name": "Wilson",
        "email_address": "bob@example.com",
        "vip_status": "Normal"
      },
      "shipping_address": {
        "address_line1": "789 Pine Rd",
        "address_line2": "",
        "address_line3": "",
        "address_line4": "",
        "city": "Chicago",
        "zip_code": "60601",
        "state": "IL",
        "country": "USA"
      },
      "billing_address": {
        "address_line1": "789 Pine Rd",
        "address_line2": "",
        "address_line3": "",
        "address_line4": "",
        "city": "Chicago",
        "zip_code": "60601",
        "state": "IL",
        "country": "USA"
      },
      "lines": [
        {
          "order_line_id": "line-001",
          "product_code": "W1234",
          "quantity": "5"
        },
        {
          "order_line_id": "line-002",
          "product_code": "G456",
          "quantity": "2.5"
        }
      ],
      "promotion_code": ""
    }'
```

### フィールド説明

| フィールド   | 形式                                             | 例          |
| ------------ | ------------------------------------------------ | ----------- |
| product_code | Widget: W + 4 桁、Gizmo: G + 3 桁                | W1234, G123 |
| quantity     | Widget: 整数 (1-1000)、Gizmo: 小数 (0.05-100.00) | 10, 3.5     |
| vip_status   | Normal または VIP                                | VIP         |
| zip_code     | 5 桁の数字                                       | 10001       |
| state        | 2 文字の州コード                                 | NY, CA      |
