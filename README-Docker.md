# 🐳 W3B2 Bridge Protocol - Docker Setup

Этот документ описывает, как запустить весь стек W3B2 Bridge Protocol с помощью Docker.

## 📋 Требования

- Docker 20.10+
- Docker Compose 2.0+
- Минимум 4GB RAM
- Минимум 2GB свободного места

## 🚀 Быстрый старт

### 1. Запуск всего стека

```bash
# Сделать скрипты исполняемыми
chmod +x scripts/*.sh

# Запустить все сервисы
./scripts/docker-start.sh
```

### 2. Режим разработки

```bash
# Запустить только инфраструктуру (Solana + Connector)
./scripts/docker-dev.sh

# В отдельных терминалах запустить:
cd w3b2-bridge-backend && npm run dev
cd w3b2-bridge-frontend && npm start
```

### 3. Остановка

```bash
# Остановить контейнеры
./scripts/docker-stop.sh

# Остановить и очистить данные
./scripts/docker-stop.sh --clean

# Полная очистка (включая образы)
./scripts/docker-stop.sh --purge
```

## 🏗️ Архитектура Docker

### Сервисы

| Сервис | Порт | Описание |
|--------|------|----------|
| `solana-validator` | 8899, 8900 | Solana тестовый валидатор |
| `bridge-program` | - | Solana программа (деплой) |
| `connector` | 50051 | Rust коннектор (gRPC) |
| `backend` | 3001, 50052 | Node.js API сервер |
| `frontend` | 3000 | React приложение |

### Volumes

- `solana-ledger` - Данные Solana валидатора
- `connector-data` - Данные коннектора (w3b2_db)
- `backend-data` - Данные backend (SQLite)

## 🔧 Конфигурация

### Переменные окружения

#### Backend (.env)
```env
SOLANA_RPC_URL=http://solana-validator:8899
CONNECTOR_GRPC_URL=connector:50051
PROGRAM_ID=3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr
DATABASE_URL=sqlite:./data/database.sqlite
ENCRYPTION_KEY=your-32-character-secret-key-here
```

#### Frontend (.env)
```env
REACT_APP_API_URL=http://localhost:3001
REACT_APP_WS_URL=ws://localhost:3001
GENERATE_SOURCEMAP=false
```

## 📊 Мониторинг

### Логи сервисов

```bash
# Все сервисы
docker-compose logs -f

# Конкретный сервис
docker-compose logs -f backend
docker-compose logs -f connector
docker-compose logs -f solana-validator
```

### Статус контейнеров

```bash
docker-compose ps
```

### Подключение к контейнеру

```bash
# Backend
docker-compose exec backend sh

# Connector
docker-compose exec connector sh

# Solana Validator
docker-compose exec solana-validator sh
```

## 🛠️ Разработка

### Локальная разработка

1. Запустите инфраструктуру:
   ```bash
   ./scripts/docker-dev.sh
   ```

2. Запустите backend локально:
   ```bash
   cd w3b2-bridge-backend
   npm install
   npm run dev
   ```

3. Запустите frontend локально:
   ```bash
   cd w3b2-bridge-frontend
   npm install
   npm start
   ```

### Пересборка образов

```bash
# Пересобрать все образы
docker-compose build --no-cache

# Пересобрать конкретный сервис
docker-compose build --no-cache backend
```

## 🔍 Отладка

### Проверка подключений

```bash
# Проверить Solana RPC
curl http://localhost:8899 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# Проверить Backend API
curl http://localhost:3001/api/health

# Проверить Frontend
curl http://localhost:3000
```

### Проблемы и решения

#### Порт уже используется
```bash
# Найти процесс, использующий порт
lsof -i :3000
lsof -i :3001
lsof -i :8899

# Остановить процесс
kill -9 <PID>
```

#### Контейнер не запускается
```bash
# Проверить логи
docker-compose logs <service-name>

# Перезапустить сервис
docker-compose restart <service-name>
```

#### Проблемы с памятью
```bash
# Увеличить лимит памяти Docker
# В Docker Desktop: Settings > Resources > Memory
```

## 📝 Полезные команды

```bash
# Очистить все неиспользуемые ресурсы
docker system prune -a

# Показать использование диска
docker system df

# Показать статистику контейнеров
docker stats

# Создать резервную копию volumes
docker run --rm -v w3b2-bridge_solana-ledger:/data -v $(pwd):/backup alpine tar czf /backup/solana-ledger-backup.tar.gz -C /data .
```

## 🚨 Безопасность

- Не используйте production ключи в Docker окружении
- Регулярно обновляйте базовые образы
- Используйте `.dockerignore` для исключения чувствительных файлов
- Не коммитьте `.env` файлы с реальными ключами

## 📚 Дополнительные ресурсы

- [Docker Documentation](https://docs.docker.com/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Solana Documentation](https://docs.solana.com/)
- [Anchor Documentation](https://www.anchor-lang.com/)
