import { create } from 'zustand';
import { LogEntry, LogLevel } from '../types';
import { getRecentLogs } from '../lib/api';

/**
 * Maximum number of logs to keep in memory
 */
const MAX_LOGS = 1000;

/**
 * Log store state interface
 */
interface LogStore {
  // Data
  logs: LogEntry[];
  
  // UI state
  isLoading: boolean;
  error: string | null;
  autoScroll: boolean;
  
  // Actions
  fetchLogs: () => Promise<void>;
  clearLogs: () => void;
  addLogEntry: (entry: LogEntry) => void;
  setAutoScroll: (enabled: boolean) => void;
  getLogsByLevel: (level: LogLevel | null) => LogEntry[];
  resetError: () => void;
}

/**
 * Log store
 */
export const useLogStore = create<LogStore>((set, get) => ({
  // Initial state
  logs: [],
  isLoading: false,
  error: null,
  autoScroll: true,

  // Fetch logs from the backend
  fetchLogs: async () => {
    try {
      set({ isLoading: true, error: null });
      const logs = await getRecentLogs();
      set({ logs, isLoading: false });
    } catch (error) {
      console.error('Failed to fetch logs:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to fetch logs', 
        isLoading: false 
      });
    }
  },

  // Clear all logs
  clearLogs: () => {
    set({ logs: [] });
  },

  // Add a new log entry (used by event listeners)
  addLogEntry: (entry) => {
    set(state => {
      const logs = [entry, ...state.logs];
      
      // Limit the number of logs to prevent memory issues
      if (logs.length > MAX_LOGS) {
        return { logs: logs.slice(0, MAX_LOGS) };
      }
      
      return { logs };
    });
  },

  // Toggle auto-scroll
  setAutoScroll: (enabled) => {
    set({ autoScroll: enabled });
  },

  // Get logs filtered by level
  getLogsByLevel: (level) => {
    const { logs } = get();
    if (!level) return logs;
    return logs.filter(log => log.level === level);
  },

  // Reset any error
  resetError: () => {
    set({ error: null });
  },
}));