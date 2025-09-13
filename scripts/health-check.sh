#!/bin/bash
set -e

echo "🏥 W3B2 Bridge Protocol - Health Check"
echo "====================================="

# Цвета для вывода
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Функция для проверки HTTP endpoint
check_http() {
    local url=$1
    local name=$2
    local expected_status=${3:-200}
    
    echo -n "Проверка $name... "
    
    if response=$(curl -s -w "%{http_code}" -o /dev/null "$url" 2>/dev/null); then
        if [ "$response" = "$expected_status" ]; then
            echo -e "${GREEN}✅ OK${NC}"
            return 0
        else
            echo -e "${RED}❌ FAIL (HTTP $response)${NC}"
            return 1
        fi
    else
        echo -e "${RED}❌ FAIL (Connection error)${NC}"
        return 1
    fi
}

# Функция для проверки gRPC endpoint
check_grpc() {
    local host=$1
    local port=$2
    local name=$3
    
    echo -n "Проверка $name... "
    
    if nc -z "$host" "$port" 2>/dev/null; then
        echo -e "${GREEN}✅ OK${NC}"
        return 0
    else
        echo -e "${RED}❌ FAIL (Connection error)${NC}"
        return 1
    fi
}

# Функция для проверки Solana RPC
check_solana_rpc() {
    echo -n "Проверка Solana RPC... "
    
    response=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
        http://localhost:8899 2>/dev/null)
    
    if echo "$response" | grep -q '"result":"ok"'; then
        echo -e "${GREEN}✅ OK${NC}"
        return 0
    else
        echo -e "${RED}❌ FAIL${NC}"
        return 1
    fi
}

# Счетчики
total_checks=0
passed_checks=0

echo ""
echo "🔍 Проверка сервисов:"
echo "-------------------"

# Проверяем Solana RPC
if check_solana_rpc; then
    ((passed_checks++))
fi
((total_checks++))

# Проверяем Solana WebSocket (порт открыт)
if check_grpc localhost 8900 "Solana WebSocket"; then
    ((passed_checks++))
fi
((total_checks++))

# Проверяем W3B2 Connector gRPC
if check_grpc localhost 50051 "W3B2 Connector gRPC"; then
    ((passed_checks++))
fi
((total_checks++))

# Проверяем Backend API
if check_http "http://localhost:3001/api/health" "Backend API"; then
    ((passed_checks++))
fi
((total_checks++))

# Проверяем Backend gRPC
if check_grpc localhost 50052 "Backend gRPC"; then
    ((passed_checks++))
fi
((total_checks++))

# Проверяем Frontend
if check_http "http://localhost:3000" "Frontend"; then
    ((passed_checks++))
fi
((total_checks++))

echo ""
echo "📊 Результаты:"
echo "-------------"
echo "Пройдено: $passed_checks/$total_checks проверок"

if [ $passed_checks -eq $total_checks ]; then
    echo -e "${GREEN}🎉 Все сервисы работают корректно!${NC}"
    exit 0
elif [ $passed_checks -gt $((total_checks / 2)) ]; then
    echo -e "${YELLOW}⚠️ Некоторые сервисы недоступны${NC}"
    exit 1
else
    echo -e "${RED}❌ Критические сервисы недоступны${NC}"
    exit 2
fi
