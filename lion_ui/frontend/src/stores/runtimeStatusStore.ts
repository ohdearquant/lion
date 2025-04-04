import { create } from 'zustand';
import { RuntimeStatus } from '../types';
import { getRuntimeStatus } from '../lib/api';

/**
 * Runtime status store state interface
 */
interface RuntimeStatusState {
  // Data
  status: RuntimeStatus;
  
  // UI state
  isLoading: boolean;
  error: string | null;
  
  // Actions
  fetchStatus: () => Promise<void>;
  updateStatus: (status: RuntimeStatus) => void;
  resetError: () => void;
}

/**
 * Default runtime status
 */
const DEFAULT_STATUS: RuntimeStatus = {
  state: 'unknown',
  uptime_seconds: 0,
  version: '',
  plugins_loaded: 0,
  agents_active: 0,
  memory_usage_mb: 0,
  cpu_usage_percent: 0,
};

/**
 * Runtime status store
 */
export const useRuntimeStatusStore = create<RuntimeStatusState>((set) => ({
  // Initial state
  status: DEFAULT_STATUS,
  isLoading: false,
  error: null,

  // Fetch runtime status from the backend
  fetchStatus: async () => {
    try {
      set({ isLoading: true, error: null });
      const status = await getRuntimeStatus();
      set({ status, isLoading: false });
    } catch (error) {
      console.error('Failed to fetch runtime status:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to fetch runtime status', 
        isLoading: false 
      });
    }
  },

  // Update runtime status (used by event listeners)
  updateStatus: (status) => {
    set({ status });
  },

  // Reset any error
  resetError: () => {
    set({ error: null });
  },
}));