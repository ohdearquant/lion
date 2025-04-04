import React, { useEffect } from 'react';
import { Sidebar } from './Sidebar';
import { MainArea } from '../main/MainArea';
import { BottomPanel } from '../panel/BottomPanel';
import { StatusBar } from '../statusbar/StatusBar';
import { useAppEvents } from '../../lib/events';
import { useRuntimeStatusStore } from '../../stores';

/**
 * Main application layout component
 * Purpose: Provides the overall IDE layout structure
 * Props: None
 * State: None (container only)
 * Children: Sidebar, MainArea, BottomPanel, StatusBar
 */
export const AppLayout: React.FC = () => {
  // Set up event listeners
  useAppEvents();
  
  // Fetch initial runtime status
  const fetchStatus = useRuntimeStatusStore(state => state.fetchStatus);
  
  useEffect(() => {
    fetchStatus();
  }, [fetchStatus]);
  
  return (
    <div className="flex flex-col h-screen bg-gray-100 dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <Sidebar />
        
        {/* Main content area */}
        <div className="flex flex-col flex-1 overflow-hidden">
          <MainArea />
          
          {/* Bottom panel */}
          <BottomPanel />
        </div>
      </div>
      
      {/* Status bar */}
      <StatusBar />
    </div>
  );
};