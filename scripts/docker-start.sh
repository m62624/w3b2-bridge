#!/bin/bash
set -e

echo "🐳 W3B2 Bridge Protocol - Docker Setup"
echo "======================================"

# Проверяем наличие Docker
if ! command -v docker &> /dev/null; then
    echo "❌ Docker не установлен. Пожалуйста, установите Docker и попробуйте снова."
    exit 1
fi

# Проверяем наличие Docker Compose
if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose не установлен. Пожалуйста, установите Docker Compose и попробуйте снова."
    exit 1
fi

# Функция для очистки
cleanup() {
    echo "🛑 Остановка и очистка контейнеров..."
    docker-compose down -v
    echo "✅ Очистка завершена"
}

# Устанавливаем обработчик сигналов
trap cleanup SIGINT SIGTERM

echo "🔧 Сборка и запуск всех сервисов..."
docker-compose up --build

echo "🎉 W3B2 Bridge Protocol запущен!"
echo ""
echo "📊 Доступные сервисы:"
echo "  • Frontend: http://localhost:3000"
echo "  • Backend API: http://localhost:3001"
echo "  • Solana RPC: http://localhost:8899"
echo "  • Solana WebSocket: ws://localhost:8900"
echo "  • Connector gRPC: localhost:50051"
echo "  • Backend gRPC: localhost:50052"
echo ""
echo "🔧 Для остановки нажмите Ctrl+C"
