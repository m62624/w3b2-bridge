#!/bin/bash
set -e

echo "🚀 W3B2 Bridge Protocol - Quick Start"
echo "===================================="

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

# Делаем скрипты исполняемыми
chmod +x scripts/*.sh

echo "🔧 Выберите режим запуска:"
echo "1) Полный стек (все сервисы)"
echo "2) Режим разработки (только инфраструктура)"
echo "3) Проверка здоровья системы"
echo "4) Остановка всех сервисов"
echo "5) Очистка системы"
echo ""

read -p "Введите номер (1-5): " choice

case $choice in
    1)
        echo "🚀 Запуск полного стека..."
        ./scripts/docker-start.sh
        ;;
    2)
        echo "🔧 Запуск в режиме разработки..."
        ./scripts/docker-dev.sh
        ;;
    3)
        echo "🏥 Проверка здоровья системы..."
        ./scripts/health-check.sh
        ;;
    4)
        echo "🛑 Остановка сервисов..."
        ./scripts/docker-stop.sh
        ;;
    5)
        echo "🧹 Очистка системы..."
        ./scripts/docker-stop.sh --clean
        ;;
    *)
        echo "❌ Неверный выбор. Пожалуйста, выберите 1-5."
        exit 1
        ;;
esac
