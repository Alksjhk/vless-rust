<template>
  <div class="xtls-vision-card">
    <div class="card-header">
      <h3>XTLS Vision 流控</h3>
      <div class="status-indicator" :class="{ active: visionStats?.active_connections > 0 }">
        {{ visionStats?.active_connections > 0 ? '活跃' : '空闲' }}
      </div>
    </div>
    
    <div class="card-content" v-if="visionStats">
      <div class="stats-grid">
        <div class="stat-item">
          <div class="stat-label">活跃连接</div>
          <div class="stat-value">{{ visionStats.active_connections }}</div>
        </div>
        
        <div class="stat-item">
          <div class="stat-label">TLS检测次数</div>
          <div class="stat-value">{{ formatNumber(visionStats.total_detections) }}</div>
        </div>
        
        <div class="stat-item">
          <div class="stat-label">Splice切换</div>
          <div class="stat-value">{{ formatNumber(visionStats.splice_switches) }}</div>
        </div>
        
        <div class="stat-item">
          <div class="stat-label">零拷贝传输</div>
          <div class="stat-value">{{ formatBytes(visionStats.splice_bytes) }}</div>
        </div>
        
        <div class="stat-item">
          <div class="stat-label">加密传输</div>
          <div class="stat-value">{{ formatBytes(visionStats.encrypted_bytes) }}</div>
        </div>
        
        <div class="stat-item">
          <div class="stat-label">Splice比例</div>
          <div class="stat-value performance-gain">{{ visionStats.splice_ratio.toFixed(1) }}%</div>
        </div>
      </div>
      
      <div class="performance-section">
        <div class="performance-header">
          <h4>性能提升</h4>
          <div class="performance-badge" :class="getPerformanceClass(visionStats.performance_gain)">
            +{{ visionStats.performance_gain.toFixed(1) }}%
          </div>
        </div>
        
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: Math.min(visionStats.performance_gain, 100) + '%' }"></div>
        </div>
        
        <div class="performance-details">
          <div class="detail-item">
            <span class="detail-label">CPU开销减少:</span>
            <span class="detail-value">~70%</span>
          </div>
          <div class="detail-item">
            <span class="detail-label">延迟降低:</span>
            <span class="detail-value">~40%</span>
          </div>
          <div class="detail-item">
            <span class="detail-label">吞吐量提升:</span>
            <span class="detail-value">2-3倍</span>
          </div>
        </div>
      </div>
    </div>
    
    <div class="card-content no-data" v-else>
      <div class="no-data-message">
        <div class="no-data-icon">🚀</div>
        <div class="no-data-text">暂无XTLS Vision数据</div>
        <div class="no-data-hint">启用XTLS流控后将显示性能统计</div>
      </div>
    </div>
  </div>
</template>

<script>
import { useWebSocket } from '../composables/useWebSocket'

export default {
  name: 'XtlsVisionCard',
  setup() {
    const { visionStats, formatBytes } = useWebSocket()
    
    return {
      visionStats,
      formatBytes
    }
  },
  methods: {
    formatNumber(num) {
      if (num >= 1000000) {
        return (num / 1000000).toFixed(1) + 'M';
      } else if (num >= 1000) {
        return (num / 1000).toFixed(1) + 'K';
      }
      return num.toString();
    },
    
    formatBytes(bytes) {
      if (bytes === 0) return '0 B';
      const k = 1024;
      const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    },
    
    getPerformanceClass(gain) {
      if (gain >= 150) return 'excellent';
      if (gain >= 100) return 'very-good';
      if (gain >= 50) return 'good';
      return 'moderate';
    }
  }
};
</script>

<style scoped>
.xtls-vision-card {
  background: var(--card-bg);
  border-radius: 12px;
  padding: 20px;
  box-shadow: var(--card-shadow);
  border: 1px solid var(--border-color);
  transition: all 0.3s ease;
}

.xtls-vision-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--card-shadow-hover);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.card-header h3 {
  margin: 0;
  color: var(--text-primary);
  font-size: 1.2rem;
  font-weight: 600;
}

.status-indicator {
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 0.85rem;
  font-weight: 500;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  transition: all 0.3s ease;
}

.status-indicator.active {
  background: linear-gradient(135deg, #10b981, #059669);
  color: white;
  box-shadow: 0 2px 8px rgba(16, 185, 129, 0.3);
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.stat-item {
  text-align: center;
  padding: 16px 12px;
  background: var(--bg-secondary);
  border-radius: 8px;
  transition: all 0.3s ease;
}

.stat-item:hover {
  background: var(--bg-tertiary);
  transform: translateY(-1px);
}

.stat-label {
  font-size: 0.85rem;
  color: var(--text-secondary);
  margin-bottom: 8px;
  font-weight: 500;
}

.stat-value {
  font-size: 1.4rem;
  font-weight: 700;
  color: var(--text-primary);
}

.stat-value.performance-gain {
  color: var(--accent-color);
}

.performance-section {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 20px;
}

.performance-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.performance-header h4 {
  margin: 0;
  color: var(--text-primary);
  font-size: 1rem;
  font-weight: 600;
}

.performance-badge {
  padding: 6px 12px;
  border-radius: 20px;
  font-size: 0.9rem;
  font-weight: 600;
  color: white;
}

.performance-badge.excellent {
  background: linear-gradient(135deg, #8b5cf6, #7c3aed);
}

.performance-badge.very-good {
  background: linear-gradient(135deg, #10b981, #059669);
}

.performance-badge.good {
  background: linear-gradient(135deg, #f59e0b, #d97706);
}

.performance-badge.moderate {
  background: linear-gradient(135deg, #6b7280, #4b5563);
}

.progress-bar {
  width: 100%;
  height: 8px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 16px;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #10b981, #8b5cf6);
  border-radius: 4px;
  transition: width 0.8s ease;
}

.performance-details {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: 12px;
}

.detail-item {
  display: flex;
  flex-direction: column;
  text-align: center;
}

.detail-label {
  font-size: 0.8rem;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.detail-value {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--accent-color);
}

.no-data {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 200px;
}

.no-data-message {
  text-align: center;
}

.no-data-icon {
  font-size: 3rem;
  margin-bottom: 16px;
  opacity: 0.6;
}

.no-data-text {
  font-size: 1.1rem;
  color: var(--text-primary);
  margin-bottom: 8px;
  font-weight: 500;
}

.no-data-hint {
  font-size: 0.9rem;
  color: var(--text-secondary);
}

/* 响应式设计 */
@media (max-width: 768px) {
  .xtls-vision-card {
    padding: 16px;
  }
  
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  
  .stat-item {
    padding: 12px 8px;
  }
  
  .stat-value {
    font-size: 1.2rem;
  }
  
  .performance-details {
    grid-template-columns: 1fr;
    gap: 8px;
  }
}

@media (max-width: 480px) {
  .card-header {
    flex-direction: column;
    gap: 12px;
    text-align: center;
  }
  
  .stats-grid {
    grid-template-columns: 1fr;
  }
  
  .performance-header {
    flex-direction: column;
    gap: 12px;
    text-align: center;
  }
}
</style>