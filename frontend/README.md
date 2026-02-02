# VLESS 监控前端

基于 Vue 3 和 Vite 构建的现代化监控页面，提供实时数据展示和交互功能。

## 技术栈

- **框架**: Vue 3 (Composition API)
- **构建工具**: Vite 6 (使用 rolldown-vite)
- **实时通信**: WebSocket 原生 API
- **图表绘制**: Canvas API
- **样式**: CSS 变量 + Glassmorphism 设计风格

## 快速开始

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run dev
```

开发服务器启动后，访问 http://localhost:5173

**特性**：
- 热模块替换 (HMR)
- 自动代理 `/api` 请求到后端 (默认 http://localhost:8443)
- 自动代理 WebSocket 连接到后端

### 构建生产版本

```bash
npm run build
```

构建产物输出到 `../static/` 目录，将被嵌入到 Rust 可执行文件中。

### 预览构建结果

```bash
npm run preview
```

## 项目结构

```
src/
├── App.vue                  # 主应用组件
├── main.js                  # 应用入口
├── components/              # Vue 组件
│   ├── StatCard.vue        # 基础统计卡片
│   ├── SpeedCard.vue       # 上传速度卡片
│   ├── DownloadCard.vue    # 下载速度卡片
│   ├── TrafficCard.vue     # 总流量卡片
│   ├── UptimeCard.vue      # 运行时长卡片
│   ├── MemoryCard.vue      # 内存使用卡片
│   ├── ConnectionsCard.vue # 活动连接卡片
│   ├── TrafficChart.vue    # 流量趋势图
│   └── ThemeToggle.vue     # 主题切换按钮
├── composables/            # 组合式函数
│   ├── useWebSocket.js     # WebSocket 连接管理
│   └── useTheme.js         # 主题切换管理
└── assets/
    └── styles/
        └── main.css        # 全局样式和 CSS 变量
```

## 组件说明

### StatCard (基础卡片)

所有统计卡片的基类，提供：
- 响应式布局
- 深色/浅色主题适配
- 玻璃态效果

### TrafficChart (流量趋势图)

使用 Canvas API 绘制的实时波形图：
- 双层渐变填充（上传/下载）
- 鼠标悬停显示详细数据
- 自适应 Y 轴刻度
- 高性能渲染 (requestAnimationFrame)

### ThemeToggle (主题切换)

- 深色/浅色主题切换
- localStorage 持久化
- 平滑过渡动画

## Composables

### useWebSocket

WebSocket 连接管理单例：
- 自动连接管理
- 消息解析和分发
- 自动降级到 API 轮询
- 历史数据缓存 (sessionStorage)

**状态**：
- `stats`: 实时统计数据
- `loading`: 加载状态
- `error`: 错误信息
- `connected`: 连接状态

**方法**：
- `reconnect()`: 重新连接
- `formatBytes()`: 字节格式化
- `getChartData()`: 获取图表数据

### useTheme

主题切换管理：
- `theme`: 当前主题 ('light' | 'dark')
- `toggleTheme()`: 切换主题

## 样式系统

### CSS 变量

```css
/* 颜色系统 */
--neon-cyan: #00f5ff
--neon-pink: #ff2d95
--bg-primary: #0a0e27
--text-primary: #ffffff

/* 字体 */
--font-size-xs: 0.75rem
--font-size-sm: 0.875rem
--font-size-base: 1rem
--font-size-lg: 1.125rem
--font-size-xl: 1.25rem
```

### Glassmorphism 效果

```css
background: rgba(255, 255, 255, 0.05);
backdrop-filter: blur(20px);
border: 1px solid rgba(255, 255, 255, 0.1);
```

## API 集成

### WebSocket 端点

```
ws://localhost:8443/api/ws
```

### HTTP 端点

```
GET /api/stats         # 获取监控数据
GET /api/speed-history # 获取速度历史
GET /api/config        # 获取配置参数
```

## 开发建议

### 添加新组件

1. 在 `src/components/` 创建 `.vue` 文件
2. 继承 `StatCard.vue` 的基础样式
3. 使用 `useWebSocket` 获取数据
4. 在 `App.vue` 中引入并使用

### 添加新 Composable

1. 在 `src/composables/` 创建 `.js` 文件
2. 导出函数返回响应式数据和方法
3. 在组件中通过 `import { useXxx } from './composables/useXxx'` 使用

### 修改样式

- 全局样式：编辑 `src/assets/styles/main.css`
- 组件样式：使用 `<style scoped>` 块
- 主题变量：修改 CSS 变量定义

## 性能优化

- **单例模式**: WebSocket 连接全局复用
- **数据缓存**: sessionStorage 缓存历史数据
- **Canvas 优化**: requestAnimationFrame 渲染
- **按需加载**: Vite 自动代码分割

## 浏览器兼容性

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

需要支持：
- WebSocket API
- Canvas API
- CSS 变量
- ES6+

## 部署说明

前端构建后静态文件输出到 `../static/`，通过 `rust-embed` 嵌入到后端可执行文件中。部署时无需前端源代码，只需编译后的可执行文件。

## 许可证

MIT License
