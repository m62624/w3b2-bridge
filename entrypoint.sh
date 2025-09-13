#!/bin/bash
set -e

echo "🚀 W3B2 Bridge Protocol - Docker Entrypoint"
echo "=========================================="

# Функция для ожидания сервиса
wait_for_service() {
    local host=$1
    local port=$2
    local service_name=$3
    local max_attempts=30
    local attempt=0
    
    echo "⏳ Ожидание $service_name на $host:$port..."
    while [ $attempt -lt $max_attempts ]; do
        if nc -z $host $port 2>/dev/null; then
            echo "✅ $service_name готов!"
            return 0
        fi
        sleep 2
        attempt=$((attempt + 1))
    done
    
    echo "❌ $service_name не запустился за отведенное время"
    return 1
}

# Запускаем Solana валидатор в фоне
echo "🔧 Запуск Solana тестового валидатора..."
solana-test-validator --reset --ledger test-ledger --log test-ledger/validator.log \
    --bind-address 0.0.0.0 --rpc-port 8899 --dynamic-port-range 8000-8020 &

VALIDATOR_PID=$!

# Ждем запуска валидатора
if ! wait_for_service localhost 8899 "Solana Validator"; then
    echo "❌ Не удалось запустить Solana валидатор"
    exit 1
fi

# Настраиваем Solana CLI
echo "⚙️ Настройка Solana CLI..."
solana config set --url http://localhost:8899

# Деплоим программу
echo "📦 Деплой Anchor программы..."
anchor build

echo "🚀 Деплой программы в блокчейн..."
solana program deploy \
  target/deploy/w3b2_bridge_program.so \
  --program-id assets/w3b2_bridge_program-keypair.json

echo "✅ Программа развернута! ID:"
cat assets/w3b2_bridge_program-keypair.json

# Запускаем коннектор в фоне
echo "🔌 Запуск W3B2 коннектора..."
cd w3b2-connector
cargo run --release --bin w3b2-connector &

CONNECTOR_PID=$!

# Ждем запуска коннектора
if ! wait_for_service localhost 50051 "W3B2 Connector"; then
    echo "❌ Не удалось запустить W3B2 коннектор"
    exit 1
fi

echo "🎉 W3B2 Bridge Protocol запущен!"
echo "📊 Доступные сервисы:"
echo "  • Solana RPC: http://localhost:8899"
echo "  • Solana WebSocket: ws://localhost:8900"
echo "  • W3B2 Connector: localhost:50051 (gRPC)"
echo ""
echo "🔧 Для остановки нажмите Ctrl+C"

# Функция для graceful shutdown
cleanup() {
    echo "🛑 Остановка сервисов..."
    kill $CONNECTOR_PID 2>/dev/null || true
    kill $VALIDATOR_PID 2>/dev/null || true
    echo "✅ Все сервисы остановлены"
    exit 0
}

# Устанавливаем обработчик сигналов
trap cleanup SIGINT SIGTERM

# Ждем завершения
wait