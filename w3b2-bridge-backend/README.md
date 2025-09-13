# W3B2 Bridge Backend

Node.js/TypeScript сервер, обеспечивающий REST API и WebSocket соединения для интеграции с W3B2 протоколом.

## Описание

Backend служит мостом между gRPC коннектором и фронтендом, предоставляя REST API для управления данными и WebSocket для real-time событий.

## Архитектура

### Компоненты
- **gRPC клиент** - подключение к коннектору
- **WebSocket сервер** - real-time события для фронтенда
- **REST API** - управление данными и операциями
- **Локальная БД** - файловое хранилище данных

### Поток данных
```
[gRPC Connector] ──gRPC──> [Backend] ──WebSocket──> [Frontend]
                              ↓
                         [REST API] ──HTTP──> [Frontend]
                              ↓
                         [File Database]
```

## Основные функции

### API Endpoints

#### Управление пользователями
- `POST /api/register-user` - регистрация пользователя
- `GET /api/user/:publicKey` - получение информации о пользователе

#### Финансирование
- `POST /api/request-funding` - запрос на финансирование
- `POST /api/approve-funding` - одобрение финансирования
- `GET /api/funding-requests` - список запросов

#### CRUD операции
- `POST /api/crud` - создание, чтение, обновление, удаление записей

#### Команды
- `POST /api/dispatch-command` - отправка команд в блокчейн

#### Сессии
- `POST /api/session/create` - создание сессии
- `POST /api/session/close` - закрытие сессии

#### События
- `GET /api/events` - получение событий
- `GET /api/events/type/:type` - события по типу
- `GET /api/events/stats` - статистика событий

### WebSocket API

#### События от сервера
- `connected` - подключение установлено
- `blockchain_event` - событие блокчейна
- `notification` - уведомление

#### События к серверу
- `subscribe_events` - подписка на события
- `unsubscribe_events` - отписка от событий

## Сервисы

### SolanaService
- Подключение к Solana блокчейну
- Отправка транзакций
- Мониторинг событий

### GrpcService
- Подключение к gRPC коннектору
- Получение событий
- Пересылка через WebSocket

### DatabaseService
- Файловое хранилище данных
- CRUD операции
- Управление событиями

### EncryptionService
- Шифрование данных
- Управление сессиями
- Безопасная передача

### WebSocketService
- Real-time соединения
- Broadcast событий
- Управление подключениями

## Конфигурация

### Переменные окружения
```env
PORT=3001
GRPC_PORT=50052
CONNECTOR_GRPC_URL=localhost:50051
FRONTEND_URL=http://localhost:3000
SOLANA_RPC_URL=https://api.devnet.solana.com
PROGRAM_ID=3LhCu6pXXdiwpvBUrFKLxCy1XQ5qyE7v6WSCLbkbS8Dr
ADMIN_PRIVATE_KEY=base64_encoded_key
DATA_DIR=./data
```

## Запуск

```bash
# Установка зависимостей
npm install

# Разработка
npm run dev

# Продакшн
npm run build
npm start
```

## Структура проекта

```
src/
├── main.ts                 # Точка входа
├── routes/
│   └── api.ts             # REST API маршруты
├── services/
│   ├── solanaService.ts   # Solana интеграция
│   ├── grpcService.ts     # gRPC клиент
│   ├── databaseService.ts # База данных
│   ├── encryptionService.ts # Шифрование
│   └── websocketService.ts # WebSocket
├── middleware/
│   └── errorHandler.ts    # Обработка ошибок
├── types/
│   └── index.ts           # Типы данных
└── utils/
    └── blockchainUtils.ts # Утилиты блокчейна
```

## Мониторинг

### Health Check
```bash
curl http://localhost:3001/api/health
```

### Статистика
```bash
curl http://localhost:3001/api/stats
```

### WebSocket статус
- Отображается в логах сервера
- Доступен через API статистики

## Безопасность

- CORS настройки для фронтенда
- Валидация входных данных
- Шифрование чувствительных данных
- Безопасное хранение ключей

## Отладка

```bash
# Просмотр логов
npm run dev

# Проверка подключений
curl http://localhost:3001/api/health
curl http://localhost:50051  # gRPC коннектор

# WebSocket тест
wscat -c ws://localhost:3001
```
