import React, { useEffect } from 'react';
import { useAgentStore } from '../../stores';
import { AgentState } from '../../types';
import { selectAgentFile } from '../../lib/api';

/**
 * Agent list component
 * Purpose: Displays a table of loaded agents with their status and action buttons
 * Props: None
 * State: Uses agentStore from Zustand
 * API Calls: unloadAgent(), loadAgent()
 * Event Listeners: None (store handles events)
 */
export const AgentListView: React.FC = () => {
  const { agents, isLoading, error, fetchAgents, loadAgent, unloadAgent } = useAgentStore();
  
  // Fetch agents on mount
  useEffect(() => {
    fetchAgents();
  }, [fetchAgents]);
  
  // Get color for agent state
  const getStateColor = (state: string): string => {
    switch (state) {
      case AgentState.RUNNING:
        return 'text-green-600 dark:text-green-400';
      case AgentState.STARTING:
        return 'text-blue-600 dark:text-blue-400';
      case AgentState.STOPPING:
        return 'text-yellow-600 dark:text-yellow-400';
      case AgentState.ERROR:
        return 'text-red-600 dark:text-red-400';
      case AgentState.STOPPED:
      default:
        return 'text-gray-600 dark:text-gray-400';
    }
  };

  // Handle loading a new agent
  const handleLoadAgent = async () => {
    try {
      const filePath = await selectAgentFile();
      if (filePath) {
        await loadAgent(filePath);
      }
    } catch (error) {
      console.error('Failed to load agent:', error);
    }
  };
  
  // Handle unloading an agent
  const handleUnloadAgent = async (agentId: string) => {
    try {
      await unloadAgent(agentId);
    } catch (error) {
      console.error(`Failed to unload agent ${agentId}:`, error);
    }
  };
  
  return (
    <div className="h-full flex flex-col">
      {/* Controls */}
      <div className="flex items-center p-2 bg-gray-100 dark:bg-gray-700 border-b border-gray-300 dark:border-gray-600">
        <button 
          onClick={() => fetchAgents()}
          className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded mr-2"
          disabled={isLoading}
        >
          Refresh
        </button>
        
        <button 
          onClick={handleLoadAgent}
          className="px-2 py-1 text-xs bg-blue-500 text-white hover:bg-blue-600 rounded"
          disabled={isLoading}
        >
          Load Agent
        </button>
        
        <div className="flex-1"></div>
        
        {isLoading && <span className="text-xs text-gray-600 dark:text-gray-400">Loading...</span>}
        {error && <span className="text-xs text-red-600 dark:text-red-400">{error}</span>}
      </div>
      
      {/* Agent list */}
      <div className="flex-1 overflow-auto">
        {agents.length === 0 ? (
          <div className="text-gray-500 dark:text-gray-400 p-4 text-center">
            No agents found
          </div>
        ) : (
          <table className="min-w-full divide-y divide-gray-300 dark:divide-gray-700">
            <thead className="bg-gray-100 dark:bg-gray-800">
              <tr>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Name
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  ID
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Type
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Version
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Status
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="bg-white dark:bg-gray-900 divide-y divide-gray-200 dark:divide-gray-800">
              {agents.map(agent => (
                <tr key={agent.id} className="hover:bg-gray-50 dark:hover:bg-gray-800">
                  <td className="px-4 py-2 text-sm">
                    {agent.name}
                  </td>
                  <td className="px-4 py-2 text-sm font-mono text-gray-600 dark:text-gray-400">
                    {agent.id}
                  </td>
                  <td className="px-4 py-2 text-sm">
                    {agent.agent_type}
                  </td>
                  <td className="px-4 py-2 text-sm">
                    {agent.version || 'N/A'}
                  </td>
                  <td className="px-4 py-2 text-sm">
                    <span className={getStateColor(agent.state)}>
                      {agent.state}
                    </span>
                  </td>
                  <td className="px-4 py-2 text-sm">
                    <button
                      onClick={() => handleUnloadAgent(agent.id)}
                      className="px-2 py-1 text-xs bg-red-500 text-white hover:bg-red-600 rounded"
                      disabled={agent.state === AgentState.STOPPING}
                    >
                      Unload
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
};