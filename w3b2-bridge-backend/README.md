# W3B2 Backend Server

Node.js/Express сервер с интеграцией Solana для W3B2 Bridge Protocol.

## 🚀 Быстрый старт

### Установка зависимостей
```bash
npm install
```

### Настройка окружения
Создайте файл `.env` в корне проекта:
```env
# Solana Configuration
SOLANA_RPC_URL=https://api.devnet.solana.com
PROGRAM_ID=W3B2Bridge111111111111111111111111111111111

# Server Configuration
PORT=3001
NODE_ENV=development

# Admin Configuration
ADMIN_PRIVATE_KEY=your_admin_private_key_here

# Database Configuration
DATA_DIR=./data

# Security
JWT_SECRET=your_jwt_secret_here
ENCRYPTION_KEY=your_encryption_key_here
```

### Запуск в режиме разработки
```bash
npm run dev
```

### Запуск в продакшене
```bash
npm run build
npm start
```

## 📡 API Endpoints

### Основные маршруты
- `GET /health` - Проверка состояния сервера
- `POST /api/register-user` - Регистрация пользователя
- `GET /api/user/:publicKey` - Получение информации о пользователе
- `POST /api/request-funding` - Запрос на финансирование
- `POST /api/approve-funding` - Одобрение финансирования
- `GET /api/funding-requests` - Список запросов на финансирование
- `POST /api/crud` - CRUD операции
- `POST /api/session/create` - Создание сессии
- `POST /api/session/close` - Закрытие сессии
- `GET /api/stats` - Статистика системы

### Примеры запросов

#### Регистрация пользователя
```bash
curl -X POST http://localhost:3001/api/register-user \
  -H "Content-Type: application/json" \
  -d '{"publicKey": "your_public_key_here"}'
```

#### Запрос на финансирование
```bash
curl -X POST http://localhost:3001/api/request-funding \
  -H "Content-Type: application/json" \
  -d '{
    "userWallet": "user_wallet_here",
    "amount": 1.5,
    "targetAdmin": "admin_public_key_here"
  }'
```

## 🔧 Конфигурация

### Переменные окружения

| Переменная | Описание | По умолчанию |
|------------|----------|--------------|
| `SOLANA_RPC_URL` | URL Solana RPC | `https://api.devnet.solana.com` |
| `PROGRAM_ID` | ID программы Solana | `W3B2Bridge111111111111111111111111111111111` |
| `PORT` | Порт сервера | `3001` |
| `NODE_ENV` | Окружение | `development` |
| `ADMIN_PRIVATE_KEY` | Приватный ключ администратора | - |
| `DATA_DIR` | Директория для данных | `./data` |

### Структура данных

Сервер использует файловую систему для хранения данных:
```
data/
├── users/          # Пользователи
├── admins/         # Администраторы
├── funding_requests/ # Запросы на финансирование
├── sessions/       # Сессии
└── records/        # Записи данных
```

## 🛡️ Безопасность

### Шифрование
- **X25519** для обмена ключами
- **AES-256** для симметричного шифрования
- **TweetNaCl** для криптографических операций

### Валидация
- Проверка публичных ключей Solana
- Валидация входных данных
- Обработка ошибок

## 📊 Мониторинг

### Логирование
- Все операции логируются в консоль
- Уровни логирования: `info`, `warn`, `error`
- Структурированные логи в JSON формате

### Метрики
- Количество пользователей
- Запросы на финансирование
- Активные сессии
- Статистика CRUD операций

## 🔄 Интеграция с Solana

### Прослушивание блокчейна
Сервер автоматически прослушивает события в блокчейне:
- Новые транзакции программы
- События регистрации пользователей
- Запросы на финансирование
- CRUD операции

### Отправка транзакций
- Регистрация администратора
- Одобрение финансирования
- Отправка команд в блокчейн

## 🐛 Отладка

### Логи
```bash
# Включить подробные логи
DEBUG=w3b2:* npm run dev
```

### Проверка подключения
```bash
# Проверка Solana
curl http://localhost:3001/health

# Проверка API
curl http://localhost:3001/api/stats
```

## 📝 Скрипты

- `npm run dev` - Запуск в режиме разработки
- `npm run build` - Сборка проекта
- `npm start` - Запуск в продакшене
- `npm run lint` - Проверка кода
- `npm run format` - Форматирование кода

## 🤝 Разработка

### Структура проекта
```
src/
├── main.ts              # Точка входа
├── services/            # Сервисы
│   ├── solanaService.ts # Solana интеграция
│   ├── encryptionService.ts # Шифрование
│   └── databaseService.ts # База данных
├── routes/              # API маршруты
│   └── api.ts
├── middleware/          # Middleware
│   └── errorHandler.ts
├── types/               # TypeScript типы
│   └── index.ts
└── utils/               # Утилиты
```

### Добавление новых endpoints
1. Создайте функцию в `routes/api.ts`
2. Добавьте типы в `types/index.ts`
3. Обновите документацию

## 📞 Поддержка

При возникновении проблем:
1. Проверьте логи сервера
2. Убедитесь в правильности конфигурации
3. Проверьте подключение к Solana сети
4. Создайте issue в репозитории
