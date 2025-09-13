#!/bin/bash
set -e

echo "🛑 W3B2 Bridge Protocol - Остановка"
echo "=================================="

# Останавливаем все контейнеры
echo "🔧 Остановка контейнеров..."
docker-compose down

# Опционально удаляем volumes
if [ "$1" = "--clean" ]; then
    echo "🧹 Очистка volumes..."
    docker-compose down -v
    echo "✅ Volumes удалены"
fi

# Опционально удаляем образы
if [ "$1" = "--purge" ]; then
    echo "🧹 Очистка образов..."
    docker-compose down -v --rmi all
    echo "✅ Образы удалены"
fi

echo "✅ W3B2 Bridge Protocol остановлен"
echo ""
echo "💡 Использование:"
echo "  ./scripts/docker-stop.sh          - Остановить контейнеры"
echo "  ./scripts/docker-stop.sh --clean  - Остановить и удалить volumes"
echo "  ./scripts/docker-stop.sh --purge  - Остановить и удалить все"
