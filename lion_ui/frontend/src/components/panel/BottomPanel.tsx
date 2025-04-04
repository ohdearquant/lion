import React, { useState } from 'react';
import { LogView } from '../logs/LogView';
import { AgentListView } from '../agents/AgentListView';

/**
 * Bottom panel component with tabs
 * Purpose: Container for tabbed panels (Logs, Agents, etc.)
 * Props: None
 * State: activeTab: string
 * Children: LogView, AgentListView
 */
export const BottomPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>('logs');
  const [isExpanded, setIsExpanded] = useState<boolean>(true);
  
  const toggleExpand = () => {
    setIsExpanded(!isExpanded);
  };
  
  if (!isExpanded) {
    return (
      <div className="h-8 bg-gray-200 dark:bg-gray-800 border-t border-gray-300 dark:border-gray-700 flex items-center px-4">
        <button 
          onClick={toggleExpand}
          className="text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
        >
          ▲ Show Panel
        </button>
      </div>
    );
  }
  
  return (
    <div className="h-64 bg-white dark:bg-gray-800 border-t border-gray-300 dark:border-gray-700 flex flex-col">
      {/* Tab header */}
      <div className="flex items-center bg-gray-200 dark:bg-gray-700 px-2">
        <button 
          onClick={() => setActiveTab('logs')}
          className={`px-4 py-2 text-sm ${
            activeTab === 'logs' 
              ? 'bg-white dark:bg-gray-800 text-blue-600 dark:text-blue-400 border-t border-l border-r border-gray-300 dark:border-gray-600' 
              : 'text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'
          }`}
        >
          Logs
        </button>
        
        <button 
          onClick={() => setActiveTab('agents')}
          className={`px-4 py-2 text-sm ${
            activeTab === 'agents' 
              ? 'bg-white dark:bg-gray-800 text-blue-600 dark:text-blue-400 border-t border-l border-r border-gray-300 dark:border-gray-600' 
              : 'text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'
          }`}
        >
          Agents
        </button>
        
        <div className="flex-1"></div>
        
        <button 
          onClick={toggleExpand}
          className="text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 px-2"
        >
          ▼
        </button>
      </div>
      
      {/* Tab content */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'logs' && <LogView />}
        {activeTab === 'agents' && <AgentListView />}
      </div>
    </div>
  );
};