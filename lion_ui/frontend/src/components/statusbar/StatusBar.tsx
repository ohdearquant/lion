import React from 'react';
import { useRuntimeStatusStore } from '../../stores';

/**
 * Status bar component
 * Purpose: Displays runtime status and other system information
 * Props: None
 * State: Uses runtimeStatusStore from Zustand
 * API Calls: None (uses store data)
 * Event Listeners: None (store handles events)
 */
export const StatusBar: React.FC = () => {
  const { status, isInitializing, error } = useRuntimeStatusStore();
  
  // Format uptime if available
  const formatUptime = (seconds?: number): string => {
    if (!seconds) return 'N/A';
    
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    
    return `${hours}h ${minutes}m ${secs}s`;
  };
  
  // Determine status indicator color
  const getStatusColor = (): string => {
    if (isInitializing) return 'bg-yellow-500';
    if (error) return 'bg-red-500';
    return status.is_running ? 'bg-green-500' : 'bg-gray-500';
  };
  
  return (
    <div className="h-8 bg-gray-200 dark:bg-gray-800 border-t border-gray-300 dark:border-gray-700 flex items-center px-4 text-sm">
      {/* Status indicator */}
      <div className="flex items-center mr-4">
        <div className={`w-3 h-3 rounded-full ${getStatusColor()} mr-2`}></div>
        <span>
          {isInitializing ? 'Initializing...' : 
           error ? 'Error' : 
           status.is_running ? 'Runtime: Running' : 'Runtime: Stopped'}
        </span>
      </div>
      
      {/* Uptime */}
      {status.is_running && status.uptime_seconds && (
        <div className="mr-4">
          <span>Uptime: {formatUptime(status.uptime_seconds)}</span>
        </div>
      )}
      
      {/* Status message */}
      <div className="flex-1 truncate">
        <span className="text-gray-600 dark:text-gray-400">
          {status.status_message || 'Ready'}
        </span>
      </div>
      
      {/* Error message if any */}
      {error && (
        <div className="text-red-500 truncate max-w-md">
          {error}
        </div>
      )}
    </div>
  );
};