import { create } from 'zustand';
import { Agent, AgentState } from '../types';
import { fetchAgents, loadAgent, unloadAgent } from '../lib/api';

/**
 * Agent store state interface
 */
interface AgentStore {
  // Data
  agents: Agent[];
  
  // UI state
  isLoading: boolean;
  error: string | null;
  selectedAgentId: string | null;
  
  // Actions
  fetchAgents: () => Promise<void>;
  loadAgent: (path: string) => Promise<string>;
  unloadAgent: (agentId: string) => Promise<void>;
  selectAgent: (id: string | null) => void;
  updateAgentStatus: (id: string, newState: AgentState) => void;
  resetError: () => void;
}

/**
 * Agent store
 */
export const useAgentStore = create<AgentStore>((set, get) => ({
  // Initial state
  agents: [],
  isLoading: false,
  error: null,
  selectedAgentId: null,

  // Fetch agents from the backend
  fetchAgents: async () => {
    try {
      set({ isLoading: true, error: null });
      const agents = await fetchAgents();
      set({ agents, isLoading: false });
    } catch (error) {
      console.error('Failed to fetch agents:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to fetch agents', 
        isLoading: false 
      });
    }
  },

  // Load an agent from a file
  loadAgent: async (path) => {
    try {
      set({ isLoading: true, error: null });
      const agent = await loadAgent(path);
      
      set((state) => ({
        agents: [...state.agents, agent],
        isLoading: false
      }));
      
      return agent.id;
    } catch (error) {
      console.error('Failed to load agent:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to load agent', 
        isLoading: false 
      });
      throw error;
    }
  },

  // Unload an agent
  unloadAgent: async (agentId) => {
    try {
      set({ isLoading: true, error: null });
      await unloadAgent(agentId);
      
      // Remove the agent from the store
      set((state) => ({
        agents: state.agents.filter(agent => agent.id !== agentId),
        isLoading: false,
        // If the selected agent was unloaded, clear the selection
        selectedAgentId: state.selectedAgentId === agentId ? null : state.selectedAgentId
      }));
    } catch (error) {
      console.error('Failed to unload agent:', error);
      set({ 
        error: error instanceof Error ? error.message : 'Failed to unload agent', 
        isLoading: false 
      });
      throw error;
    }
  },

  // Select an agent
  selectAgent: (id) => {
    set({ selectedAgentId: id });
  },

  // Update agent status (used by event listeners)
  updateAgentStatus: (id, newState) => {
    const { agents } = get();
    
    const updatedAgents = agents.map(agent => 
      agent.id === id ? { ...agent, state: newState } : agent
    );
    
    set({ agents: updatedAgents });
  },

  // Reset any error
  resetError: () => {
    set({ error: null });
  },
}));