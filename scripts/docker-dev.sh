#!/bin/bash
set -e

echo "🐳 W3B2 Bridge Protocol - Development Mode"
echo "=========================================="

# Проверяем наличие Docker
if ! command -v docker &> /dev/null; then
    echo "❌ Docker не установлен. Пожалуйста, установите Docker и попробуйте снова."
    exit 1
fi

echo "🔧 Запуск в режиме разработки..."
echo "Этот режим запускает только Solana валидатор и коннектор."
echo "Backend и Frontend нужно запускать локально для разработки."
echo ""

# Запускаем только валидатор и коннектор
docker-compose up solana-validator bridge-program connector

echo "🎉 Инфраструктура готова для разработки!"
echo ""
echo "📊 Доступные сервисы:"
echo "  • Solana RPC: http://localhost:8899"
echo "  • Solana WebSocket: ws://localhost:8900"
echo "  • Connector gRPC: localhost:50051"
echo ""
echo "🔧 Теперь вы можете запустить:"
echo "  • Backend: cd w3b2-bridge-backend && npm run dev"
echo "  • Frontend: cd w3b2-bridge-frontend && npm start"
