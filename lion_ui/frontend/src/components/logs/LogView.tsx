import React, { useEffect, useRef, useState } from 'react';
import { useLogStore } from '../../stores';
import { LogLevel } from '../../types';

/**
 * Log viewer component
 * Purpose: Displays system logs with auto-scrolling
 * Props: None
 * State: Uses logStore from Zustand
 * API Calls: None (uses store data)
 * Event Listeners: None (store handles events)
 * Features: Auto-scrolling, clear button, level-based coloring
 */
export const LogView: React.FC = () => {
  const { logs, isLoading, error, fetchRecentLogs, clearLogs } = useLogStore();
  const [filter, setFilter] = useState<LogLevel | null>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const logContainerRef = useRef<HTMLDivElement>(null);
  
  // Fetch logs on mount
  useEffect(() => {
    fetchRecentLogs();
  }, [fetchRecentLogs]);
  
  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);
  
  // Get logs filtered by level if filter is active
  const filteredLogs = filter ? logs.filter(log => log.level === filter) : logs;
  
  // Get color for log level
  const getLevelColor = (level: string): string => {
    switch (level) {
      case 'ERROR':
        return 'text-red-600 dark:text-red-400';
      case 'WARN':
        return 'text-yellow-600 dark:text-yellow-400';
      case 'INFO':
        return 'text-blue-600 dark:text-blue-400';
      case 'DEBUG':
        return 'text-green-600 dark:text-green-400';
      case 'TRACE':
        return 'text-gray-600 dark:text-gray-400';
      default:
        return 'text-gray-800 dark:text-gray-200';
    }
  };
  
  return (
    <div className="h-full flex flex-col">
      {/* Controls */}
      <div className="flex items-center p-2 bg-gray-100 dark:bg-gray-700 border-b border-gray-300 dark:border-gray-600">
        <div className="flex space-x-2 mr-4">
          <button 
            onClick={() => clearLogs()}
            className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded"
            disabled={isLoading}
          >
            Clear
          </button>
          
          <button 
            onClick={() => fetchRecentLogs()}
            className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded"
            disabled={isLoading}
          >
            Refresh
          </button>
        </div>
        
        <div className="flex items-center space-x-2 mr-4">
          <span className="text-xs">Filter:</span>
          <select 
            value={filter || ''}
            onChange={(e) => setFilter(e.target.value ? e.target.value as LogLevel : null)}
            className="text-xs p-1 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded"
          >
            <option value="">All Levels</option>
            {Object.values(LogLevel).map(level => (
              <option key={level} value={level}>{level}</option>
            ))}
          </select>
        </div>
        
        <div className="flex items-center">
          <label className="flex items-center text-xs">
            <input 
              type="checkbox" 
              checked={autoScroll} 
              onChange={() => setAutoScroll(!autoScroll)}
              className="mr-1"
            />
            Auto-scroll
          </label>
        </div>
        
        <div className="flex-1"></div>
        
        {isLoading && <span className="text-xs text-gray-600 dark:text-gray-400">Loading...</span>}
        {error && <span className="text-xs text-red-600 dark:text-red-400">{error}</span>}
      </div>
      
      {/* Log entries */}
      <div 
        ref={logContainerRef}
        className="flex-1 overflow-auto font-mono text-xs p-2 bg-white dark:bg-gray-900"
      >
        {filteredLogs.length === 0 ? (
          <div className="text-gray-500 dark:text-gray-400 p-4 text-center">
            No logs to display
          </div>
        ) : (
          filteredLogs.map(log => (
            <div key={log.id} className="mb-1 leading-tight">
              <span className="text-gray-500 dark:text-gray-400">[{log.timestamp}]</span>{' '}
              <span className={getLevelColor(log.level)}>[{log.level}]</span>{' '}
              <span className="text-gray-700 dark:text-gray-300">[{log.source}]</span>{' '}
              <span>{log.message}</span>
            </div>
          ))
        )}
      </div>
    </div>
  );
};