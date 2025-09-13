# W3B2 Bridge Protocol - Docker Management
# =======================================

.PHONY: help build up down restart logs clean dev prod status

# Цвета для вывода
GREEN=\033[0;32m
YELLOW=\033[1;33m
RED=\033[0;31m
NC=\033[0m # No Color

help: ## Показать справку
	@echo "$(GREEN)W3B2 Bridge Protocol - Docker Management$(NC)"
	@echo "=============================================="
	@echo ""
	@echo "$(YELLOW)Основные команды:$(NC)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "$(YELLOW)Примеры использования:$(NC)"
	@echo "  make up          # Запустить все сервисы"
	@echo "  make dev         # Режим разработки"
	@echo "  make logs        # Показать логи"
	@echo "  make down        # Остановить сервисы"

build: ## Собрать все Docker образы
	@echo "$(GREEN)🔧 Сборка Docker образов...$(NC)"
	docker-compose build --no-cache

up: ## Запустить все сервисы
	@echo "$(GREEN)🚀 Запуск W3B2 Bridge Protocol...$(NC)"
	@chmod +x scripts/*.sh
	./scripts/docker-start.sh

dev: ## Запустить в режиме разработки (только инфраструктура)
	@echo "$(GREEN)🔧 Запуск в режиме разработки...$(NC)"
	@chmod +x scripts/*.sh
	./scripts/docker-dev.sh

down: ## Остановить все сервисы
	@echo "$(YELLOW)🛑 Остановка сервисов...$(NC)"
	docker-compose down

restart: ## Перезапустить все сервисы
	@echo "$(YELLOW)🔄 Перезапуск сервисов...$(NC)"
	docker-compose restart

logs: ## Показать логи всех сервисов
	@echo "$(GREEN)📋 Логи сервисов:$(NC)"
	docker-compose logs -f

logs-backend: ## Показать логи backend
	docker-compose logs -f backend

logs-connector: ## Показать логи connector
	docker-compose logs -f connector

logs-validator: ## Показать логи Solana validator
	docker-compose logs -f solana-validator

status: ## Показать статус контейнеров
	@echo "$(GREEN)📊 Статус контейнеров:$(NC)"
	docker-compose ps

clean: ## Остановить и удалить контейнеры
	@echo "$(YELLOW)🧹 Очистка контейнеров...$(NC)"
	docker-compose down -v

purge: ## Полная очистка (контейнеры + образы)
	@echo "$(RED)🧹 Полная очистка...$(NC)"
	docker-compose down -v --rmi all

shell-backend: ## Подключиться к shell backend контейнера
	docker-compose exec backend sh

shell-connector: ## Подключиться к shell connector контейнера
	docker-compose exec connector sh

shell-validator: ## Подключиться к shell validator контейнера
	docker-compose exec solana-validator sh

test: ## Запустить тесты
	@echo "$(GREEN)🧪 Запуск тестов...$(NC)"
	docker-compose exec backend npm test
	docker-compose exec connector cargo test

health: ## Проверить здоровье сервисов
	@echo "$(GREEN)🏥 Проверка здоровья сервисов...$(NC)"
	@echo "Solana RPC:"
	@curl -s http://localhost:8899 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' || echo "$(RED)❌ Solana RPC недоступен$(NC)"
	@echo ""
	@echo "Backend API:"
	@curl -s http://localhost:3001/api/health || echo "$(RED)❌ Backend API недоступен$(NC)"
	@echo ""
	@echo "Frontend:"
	@curl -s http://localhost:3000 > /dev/null && echo "$(GREEN)✅ Frontend доступен$(NC)" || echo "$(RED)❌ Frontend недоступен$(NC)"

# Команды для разработки
install-backend: ## Установить зависимости backend
	cd w3b2-bridge-backend && npm install

install-frontend: ## Установить зависимости frontend
	cd w3b2-bridge-frontend && npm install

install: install-backend install-frontend ## Установить все зависимости

dev-backend: ## Запустить backend в режиме разработки
	cd w3b2-bridge-backend && npm run dev

dev-frontend: ## Запустить frontend в режиме разработки
	cd w3b2-bridge-frontend && npm start

# Команды для мониторинга
monitor: ## Показать мониторинг ресурсов
	docker stats

volumes: ## Показать информацию о volumes
	docker volume ls | grep w3b2

backup: ## Создать резервную копию данных
	@echo "$(GREEN)💾 Создание резервной копии...$(NC)"
	mkdir -p backups
	docker run --rm -v w3b2-bridge_solana-ledger:/data -v $(PWD)/backups:/backup alpine tar czf /backup/solana-ledger-$(shell date +%Y%m%d-%H%M%S).tar.gz -C /data .
	docker run --rm -v w3b2-bridge_connector-data:/data -v $(PWD)/backups:/backup alpine tar czf /backup/connector-data-$(shell date +%Y%m%d-%H%M%S).tar.gz -C /data .
	docker run --rm -v w3b2-bridge_backend-data:/data -v $(PWD)/backups:/backup alpine tar czf /backup/backend-data-$(shell date +%Y%m%d-%H%M%S).tar.gz -C /data .
	@echo "$(GREEN)✅ Резервная копия создана в папке backups/$(NC)"

# Команды для отладки
debug: ## Запустить в режиме отладки
	@echo "$(YELLOW)🐛 Режим отладки...$(NC)"
	docker-compose -f docker-compose.yml -f docker-compose.debug.yml up

# Команды для production
prod: ## Запустить в production режиме
	@echo "$(GREEN)🏭 Запуск в production режиме...$(NC)"
	docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d

# Команды для обновления
update: ## Обновить все образы
	@echo "$(GREEN)🔄 Обновление образов...$(NC)"
	docker-compose pull
	docker-compose build --no-cache

# Команды для безопасности
scan: ## Сканировать образы на уязвимости
	@echo "$(GREEN)🔍 Сканирование образов...$(NC)"
	docker run --rm -v /var/run/docker.sock:/var/run/docker.sock \
		-v $(PWD):/workspace \
		aquasec/trivy image w3b2-bridge_backend:latest

# Команды для документации
docs: ## Показать документацию
	@echo "$(GREEN)📚 Документация W3B2 Bridge Protocol$(NC)"
	@echo "=========================================="
	@echo ""
	@echo "🐳 Docker:"
	@echo "  README-Docker.md - Полная документация по Docker"
	@echo ""
	@echo "🔧 Разработка:"
	@echo "  README.md - Основная документация"
	@echo "  w3b2-bridge-program/README.md - Solana программа"
	@echo "  w3b2-connector/README.md - Коннектор"
	@echo "  w3b2-bridge-backend/README.md - Backend API"
	@echo "  w3b2-bridge-frontend/README.md - Frontend"
	@echo ""
	@echo "📊 API:"
	@echo "  http://localhost:3001/api/health - Health check"
	@echo "  http://localhost:3001/api/stats - Статистика"
	@echo "  http://localhost:3000 - Frontend интерфейс"
