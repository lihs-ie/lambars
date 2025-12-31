# ============================================================================
# order-taking-server Dockerfile
# ============================================================================
# マルチステージビルドで最小のイメージサイズを実現
#
# 使用方法:
#   # functional-rusty ルートディレクトリから実行
#   docker build -t order-taking:latest -f samples/order/Dockerfile .
#   docker run -p 8080:8080 order-taking:latest
#
# ============================================================================

# ============================================================================
# Stage 1: Builder
# ============================================================================
FROM rust:1.92-alpine AS builder

# ビルド依存のインストール
RUN apk add --no-cache musl-dev

# ワークスペース構造を再現
WORKDIR /workspace

# Cargo.toml と Cargo.lock をコピー（依存関係キャッシュ用）
COPY Cargo.toml Cargo.lock ./

# functional-rusty 本体をコピー
COPY src ./src
COPY functional-rusty-derive ./functional-rusty-derive

# サンプルプロジェクトをコピー
COPY samples/order ./samples/order

# リリースビルド
WORKDIR /workspace/samples/order
RUN cargo build --release --bin order-taking-server

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM alpine:3.20 AS runtime

# セキュリティ: 非 root ユーザー作成
RUN addgroup -S app && adduser -S app -G app

# バイナリをコピー
COPY --from=builder /workspace/samples/order/target/release/order-taking-server /usr/local/bin/

# ユーザー切り替え
USER app

# ポート公開
EXPOSE 8080

# 環境変数のデフォルト
ENV RUST_LOG=order_taking_server=info

# エントリポイント
ENTRYPOINT ["order-taking-server"]
