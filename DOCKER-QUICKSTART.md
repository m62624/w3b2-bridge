# 🐳 W3B2 Bridge Protocol - Быстрый старт с Docker

## ⚡ Мгновенный запуск

```bash
# Сделать скрипт исполняемым и запустить
chmod +x start.sh
./start.sh
```

## 🎯 Основные команды

### Запуск
```bash
# Полный стек (все сервисы)
make up

# Режим разработки (только инфраструктура)
make dev

# Проверка здоровья
make health
```

### Управление
```bash
# Остановить
make down

# Перезапустить
make restart

# Логи
make logs

# Статус
make status
```

### Разработка
```bash
# Установить зависимости
make install

# Запустить backend локально
make dev-backend

# Запустить frontend локально
make dev-frontend
```

## 📊 Доступные сервисы

| Сервис | URL | Описание |
|--------|-----|----------|
| **Frontend** | http://localhost:3000 | React приложение |
| **Backend API** | http://localhost:3001 | REST API |
| **Solana RPC** | http://localhost:8899 | Блокчейн RPC |
| **Solana WS** | ws://localhost:8900 | WebSocket |
| **Connector gRPC** | localhost:50051 | Rust коннектор |
| **Backend gRPC** | localhost:50052 | Backend gRPC |

## 🔧 Режимы работы

### 1. Полный стек (Production)
```bash
make up
```
Запускает все сервисы в Docker контейнерах.

### 2. Режим разработки
```bash
make dev
```
Запускает только инфраструктуру (Solana + Connector), 
Backend и Frontend запускаются локально.

### 3. Отладка
```bash
make debug
```
Запускает с включенными debug логами.

## 🏥 Мониторинг

### Проверка здоровья
```bash
make health
```

### Логи сервисов
```bash
# Все сервисы
make logs

# Конкретный сервис
make logs-backend
make logs-connector
make logs-validator
```

### Статистика ресурсов
```bash
make monitor
```

## 🛠️ Отладка

### Подключение к контейнерам
```bash
make shell-backend
make shell-connector
make shell-validator
```

### Проверка подключений
```bash
# Solana RPC
curl http://localhost:8899 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# Backend API
curl http://localhost:3001/api/health

# Frontend
curl http://localhost:3000
```

## 🧹 Очистка

### Остановка сервисов
```bash
make down
```

### Очистка данных
```bash
make clean
```

### Полная очистка
```bash
make purge
```

## 📚 Дополнительно

- **Полная документация**: `README-Docker.md`
- **Справка по командам**: `make help`
- **API документация**: http://localhost:3001/api/health

## 🚨 Решение проблем

### Порт уже используется
```bash
# Найти процесс
lsof -i :3000
lsof -i :3001
lsof -i :8899

# Остановить процесс
kill -9 <PID>
```

### Контейнер не запускается
```bash
# Проверить логи
make logs-backend

# Перезапустить
make restart
```

### Проблемы с памятью
Увеличьте лимит памяти Docker в настройках Docker Desktop.

## 🎉 Готово!

После запуска откройте http://localhost:3000 в браузере для доступа к интерфейсу W3B2 Bridge Protocol.
