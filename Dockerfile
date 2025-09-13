# Основной Dockerfile для полного стека W3B2 Bridge
FROM debian:bullseye-slim

ARG SOLANA_VERSION=v2.1.0
ARG ANCHOR_VERSION=0.31.1
ARG NODE_VERSION=18

ENV DEBIAN_FRONTEND=noninteractive
ENV SOLANA_VERSION_ENV=${SOLANA_VERSION}
ENV ANCHOR_VERSION_ENV=${ANCHOR_VERSION}

# Устанавливаем базовые пакеты
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    python3 \
    ca-certificates \
    wget \
    gnupg \
    && rm -rf /var/lib/apt/lists/*

# Устанавливаем Node.js
RUN curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION}.x | bash - \
    && apt-get install -y nodejs

# Устанавливаем Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Устанавливаем Solana CLI
RUN curl -sSfL https://release.solana.com/${SOLANA_VERSION_ENV}/install | sh
ENV PATH="/root/.local/share/solana/install/active_release/bin:${PATH}"

# Устанавливаем Anchor CLI
RUN cargo install anchor-cli@${ANCHOR_VERSION_ENV} --locked --force

# Создаем рабочую директорию
WORKDIR /project

# Копируем исходники проекта
COPY . .

# Устанавливаем зависимости для backend
WORKDIR /project/w3b2-bridge-backend
RUN npm ci --only=production

# Устанавливаем зависимости для frontend
WORKDIR /project/w3b2-bridge-frontend
RUN npm ci

# Возвращаемся в корень проекта
WORKDIR /project

# Собираем Anchor программу из корневой директории
RUN anchor build

# Генерируем IDL
RUN anchor idl build -o target/idl/w3b2_bridge_program.json

# Собираем Rust коннектор
RUN cargo build --release

# Создаем директории для данных
RUN mkdir -p w3b2_db test-ledger w3b2-bridge-backend/data

# Копируем конфигурационные файлы
COPY config.toml ./
COPY w3b2-connector/config.toml ./w3b2-connector/
COPY w3b2-bridge-backend/dev.env ./w3b2-bridge-backend/.env
COPY w3b2-bridge-frontend/dev.env ./w3b2-bridge-frontend/.env

# Создаем скрипт запуска всего стека
RUN echo '#!/bin/bash\n\
set -e\n\
\n\
echo "🚀 Запуск W3B2 Bridge Protocol..."\n\
\n\
# Функция для ожидания сервиса\n\
wait_for_service() {\n\
    local host=$1\n\
    local port=$2\n\
    local service_name=$3\n\
    \n\
    echo "⏳ Ожидание $service_name на $host:$port..."\n\
    while ! nc -z $host $port; do\n\
        sleep 1\n\
    done\n\
    echo "✅ $service_name готов!"\n\
}\n\
\n\
# Запускаем Solana валидатор в фоне\n\
echo "🔧 Запуск Solana тестового валидатора..."\n\
solana-test-validator --reset --ledger test-ledger --log test-ledger/validator.log \\\n\
    --bind-address 0.0.0.0 --rpc-port 8899 --dynamic-port-range 8000-8020 &\n\
\n\
VALIDATOR_PID=$!\n\
\n\
# Ждем запуска валидатора\n\
wait_for_service localhost 8899 "Solana Validator"\n\
\n\
# Настраиваем Solana CLI\n\
echo "⚙️ Настройка Solana CLI..."\n\
solana config set --url http://localhost:8899\n\
\n\
# Деплоим программу\n\
echo "📦 Деплой Anchor программы..."\n\
anchor build\n\
\n\
echo "🚀 Деплой программы в блокчейн..."\n\
solana program deploy \\\n\
  target/deploy/w3b2_bridge_program.so \\\n\
  --program-id assets/w3b2_bridge_program-keypair.json\n\
\n\
echo "✅ Программа развернута! ID:"\n\
cat assets/w3b2_bridge_program-keypair.json\n\
\n\
echo "📋 IDL файл создан:"\n\
ls -la w3b2-bridge-program/target/idl/\n\
\n\
# Запускаем коннектор в фоне\n\
echo "🔌 Запуск W3B2 коннектора..."\n\
cd w3b2-connector\n\
cargo run --release --bin w3b2-connector &\n\
\n\
CONNECTOR_PID=$!\n\
\n\
# Ждем запуска коннектора\n\
wait_for_service localhost 50051 "W3B2 Connector"\n\
\n\
# Запускаем backend в фоне\n\
echo "🌐 Запуск W3B2 Backend..."\n\
cd ../w3b2-bridge-backend\n\
npm start &\n\
\n\
BACKEND_PID=$!\n\
\n\
# Ждем запуска backend\n\
wait_for_service localhost 3001 "W3B2 Backend"\n\
\n\
# Запускаем frontend в фоне\n\
echo "🎨 Запуск W3B2 Frontend..."\n\
cd ../w3b2-bridge-frontend\n\
npm start &\n\
\n\
FRONTEND_PID=$!\n\
\n\
# Ждем запуска frontend\n\
wait_for_service localhost 3000 "W3B2 Frontend"\n\
\n\
echo "🎉 W3B2 Bridge Protocol полностью запущен!"\n\
echo "📊 Статус сервисов:"\n\
echo "  • Solana Validator: http://localhost:8899"\n\
echo "  • W3B2 Connector: localhost:50051 (gRPC)"\n\
echo "  • W3B2 Backend: http://localhost:3001"\n\
echo "  • W3B2 Frontend: http://localhost:3000"\n\
echo ""\n\
echo "🔧 Для остановки нажмите Ctrl+C"\n\
\n\
# Функция для graceful shutdown\n\
cleanup() {\n\
    echo "🛑 Остановка сервисов..."\n\
    kill $FRONTEND_PID 2>/dev/null || true\n\
    kill $BACKEND_PID 2>/dev/null || true\n\
    kill $CONNECTOR_PID 2>/dev/null || true\n\
    kill $VALIDATOR_PID 2>/dev/null || true\n\
    echo "✅ Все сервисы остановлены"\n\
    exit 0\n\
}\n\
\n\
# Устанавливаем обработчик сигналов\n\
trap cleanup SIGINT SIGTERM\n\
\n\
# Ждем завершения\n\
wait\n\
' > /start-all.sh && chmod +x /start-all.sh

# Устанавливаем netcat для проверки портов
RUN apt-get update && apt-get install -y netcat-openbsd && rm -rf /var/lib/apt/lists/*

# Экспонируем порты
EXPOSE 8899 8900 3000 3001 50051 50052

# Запускаем весь стек
CMD ["/start-all.sh"]