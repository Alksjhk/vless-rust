# Makefile for vless-rust
# 完整构建流程：前端 + 后端

.PHONY: all release dev clean build-release build-dev frontend frontend-dev frontend-install clean-frontend help

# 默认目标：完整构建（前端 + 后端 release）
all: frontend release

# 前端安装依赖
frontend-install:
	@echo "安装前端依赖..."
	cd frontend && npm install

# 前端构建（生产版本）
frontend:
	@echo "正在构建前端..."
	cd frontend && npm run build
	@echo "前端构建完成！输出到 static/ 目录"

# 前端开发服务器
frontend-dev:
	@echo "启动前端开发服务器..."
	cd frontend && npm run dev

# 完整构建（前端 + 后端 release）
full: frontend release
	@echo "完整构建完成！"

# 编译 release 版本（优化）并复制到根目录
release:
	@echo "正在编译 Release 版本..."
	cargo build --release
	@echo "复制可执行文件到项目根目录..."
	cp target/release/vless-rust.exe ./vless-rust.exe 2>/dev/null || cp target/release/vless-rust ./vless-rust
	@echo "完成！可执行文件: ./vless-rust"

# 编译 debug 版本并复制到根目录
dev:
	@echo "正在编译 Debug 版本..."
	cargo build
	@echo "复制可执行文件到项目根目录..."
	cp target/debug/vless-rust.exe ./vless-rust-debug.exe 2>/dev/null || cp target/debug/vless-rust ./vless-rust-debug
	@echo "完成！可执行文件: ./vless-rust-debug"

# 仅编译，不复制（使用 Cargo 默认行为）
build-release:
	cargo build --release

build-dev:
	cargo build

# 清理后端编译产物
clean:
	cargo clean
	@echo "已清理 target 目录"

# 清理前端构建产物
clean-frontend:
	@echo "清理前端构建产物..."
	rm -rf frontend/dist frontend/node_modules/.vite
	@echo "前端清理完成"

# 清理所有编译产物（前端 + 后端）
clean-all: clean clean-frontend
	@echo "删除根目录的可执行文件..."
	rm -f ./vless-rust ./vless-rust.exe ./vless-rust-debug ./vless-rust-debug.exe
	@echo "全部清理完成"

# 运行服务器
run: release
	./vless-rust.exe 2>/dev/null || ./vless-rust

# 运行 debug 版本
run-dev: dev
	./vless-rust-debug.exe 2>/dev/null || ./vless-rust-debug

# 显示帮助信息
help:
	@echo "vless-rust 构建命令："
	@echo ""
	@echo "完整构建："
	@echo "  make              - 完整构建（前端 + 后端 release）"
	@echo "  make full         - 完整构建（前端 + 后端 release）"
	@echo ""
	@echo "前端相关："
	@echo "  make frontend     - 构建前端（生产版本）"
	@echo "  make frontend-dev - 启动前端开发服务器"
	@echo "  make frontend-install - 安装前端依赖"
	@echo ""
	@echo "后端相关："
	@echo "  make release      - 编译后端 release 版本并复制到根目录"
	@echo "  make dev          - 编译后端 debug 版本并复制到根目录"
	@echo ""
	@echo "运行："
	@echo "  make run          - 编译并运行 release 版本"
	@echo "  make run-dev      - 编译并运行 debug 版本"
	@echo ""
	@echo "清理："
	@echo "  make clean        - 清理后端 target 目录"
	@echo "  make clean-frontend - 清理前端构建产物"
	@echo "  make clean-all    - 清理所有编译产物（前端 + 后端）"
	@echo ""
	@echo "其他："
	@echo "  make help         - 显示此帮助信息"
