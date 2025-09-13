#!/bin/bash

# Скрипт для запуска всех компонентов W3B2 Bridge

echo "🚀 Запуск W3B2 Bridge системы..."

# Функция для проверки порта
check_port() {
    if lsof -Pi :$1 -sTCP:LISTEN -t >/dev/null ; then
        echo "❌ Порт $1 уже занят"
        exit 1
    fi
}

# Проверяем порты
echo "🔍 Проверка портов..."
check_port 50051  # gRPC коннектор
check_port 3001   # Backend
check_port 3000   # Frontend

# Создаем директории для логов
mkdir -p logs

echo "📦 Установка зависимостей..."

# Установка зависимостей бэкенда
echo "📦 Установка зависимостей бэкенда..."
cd w3b2-bridge-backend
npm install
cd ..

# Установка зависимостей фронтенда
echo "📦 Установка зависимостей фронтенда..."
cd w3b2-bridge-frontend
npm install
cd ..

echo "🔧 Сборка коннектора..."
cd w3b2-connector
cargo build --release
cd ..

echo "🚀 Запуск компонентов..."

# Запуск gRPC коннектора в фоне
echo "🔌 Запуск gRPC коннектора..."
cd w3b2-connector
cargo run > ../logs/connector.log 2>&1 &
CONNECTOR_PID=$!
cd ..

# Ждем запуска коннектора
sleep 5

# Запуск бэкенда в фоне
echo "🖥️ Запуск бэкенда..."
cd w3b2-bridge-backend
npm run dev > ../logs/backend.log 2>&1 &
BACKEND_PID=$!
cd ..

# Ждем запуска бэкенда
sleep 5

# Запуск фронтенда в фоне
echo "🌐 Запуск фронтенда..."
cd w3b2-bridge-frontend
npm start > ../logs/frontend.log 2>&1 &
FRONTEND_PID=$!
cd ..

echo "✅ Все компоненты запущены!"
echo ""
echo "📊 Статус компонентов:"
echo "  🔌 gRPC коннектор: PID $CONNECTOR_PID (порт 50051)"
echo "  🖥️ Backend: PID $BACKEND_PID (порт 3001)"
echo "  🌐 Frontend: PID $FRONTEND_PID (порт 3000)"
echo ""
echo "📝 Логи:"
echo "  📄 Коннектор: logs/connector.log"
echo "  📄 Бэкенд: logs/backend.log"
echo "  📄 Фронтенд: logs/frontend.log"
echo ""
echo "🌐 Откройте http://localhost:3000 в браузере"
echo ""
echo "🛑 Для остановки нажмите Ctrl+C"

# Функция для остановки всех процессов
cleanup() {
    echo ""
    echo "🛑 Остановка всех компонентов..."
    kill $CONNECTOR_PID 2>/dev/null
    kill $BACKEND_PID 2>/dev/null
    kill $FRONTEND_PID 2>/dev/null
    echo "✅ Все компоненты остановлены"
    exit 0
}

# Обработка сигналов
trap cleanup SIGINT SIGTERM

# Ждем завершения
wait
