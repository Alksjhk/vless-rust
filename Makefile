# Makefile for vless-rust
# 纯 VLESS 服务器构建

.PHONY: all release dev clean build-release build-dev help

# 默认目标：编译 release 版本
all: release

# 编译 release 版本（优化）并复制到根目录
release:
	@echo "正在编译 Release 版本..."
	cargo build --release
	@echo "复制可执行文件到项目根目录..."
	cp target/release/vless.exe ./vless.exe 2>/dev/null || cp target/release/vless ./vless
	@echo "完成！可执行文件: ./vless"

# 编译 debug 版本并复制到根目录
dev:
	@echo "正在编译 Debug 版本..."
	cargo build
	@echo "复制可执行文件到项目根目录..."
	cp target/debug/vless.exe ./vless-debug.exe 2>/dev/null || cp target/debug/vless ./vless-debug
	@echo "完成！可执行文件: ./vless-debug"

# 仅编译，不复制（使用 Cargo 默认行为）
build-release:
	cargo build --release

build-dev:
	cargo build

# 清理编译产物
clean:
	cargo clean
	@echo "已清理 target 目录"
	@echo "删除根目录的可执行文件..."
	rm -f ./vless ./vless.exe ./vless-debug ./vless-debug.exe
	@echo "全部清理完成"

# 运行服务器
run: release
	./vless.exe 2>/dev/null || ./vless

# 运行 debug 版本
run-dev: dev
	./vless-debug.exe 2>/dev/null || ./vless-debug

# 显示帮助信息
help:
	@echo "vless-rust 构建命令："
	@echo ""
	@echo "编译："
	@echo "  make all         - 编译 release 版本（默认）"
	@echo "  make release     - 编译 release 版本并复制到根目录"
	@echo "  make dev         - 编译 debug 版本并复制到根目录"
	@echo ""
	@echo "运行："
	@echo "  make run         - 编译并运行 release 版本"
	@echo "  make run-dev     - 编译并运行 debug 版本"
	@echo ""
	@echo "清理："
	@echo "  make clean       - 清理所有编译产物"
	@echo ""
	@echo "其他："
	@echo "  make help        - 显示此帮助信息"
