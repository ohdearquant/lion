import { create } from 'zustand';
import { 
  fetchWorkflowInstances, 
  startWorkflowInstance, 
  cancelWorkflowInstance,
  getWorkflowInstanceDetails
} from '../lib/api';

export type WorkflowInstanceStatus = 
  'PENDING' | 
  'RUNNING' | 
  'COMPLETED' | 
  'FAILED' | 
  'CANCELLED';

export interface WorkflowInstance {
  instanceId: string;
  workflowId: string;
  workflowName: string;
  status: WorkflowInstanceStatus;
  startTime: string;
  endTime?: string;
  input: string;
  output?: string;
  error?: string;
  progress?: number;
  currentNodeId?: string;
}

export interface WorkflowInstanceDetails extends WorkflowInstance {
  logs: {
    timestamp: string;
    level: string;
    message: string;
  }[];
  nodeStates: {
    nodeId: string;
    status: 'PENDING' | 'RUNNING' | 'COMPLETED' | 'FAILED' | 'SKIPPED';
    startTime?: string;
    endTime?: string;
    output?: string;
    error?: string;
  }[];
}

interface WorkflowInstanceState {
  instances: WorkflowInstance[];
  selectedInstanceId: string | null;
  instanceDetails: WorkflowInstanceDetails | null;
  isLoading: boolean;
  error: string | null;
  
  // Actions
  fetchInstances: (filter?: { status?: WorkflowInstanceStatus }) => Promise<void>;
  startInstance: (workflowId: string, input: string) => Promise<string>;
  cancelInstance: (instanceId: string) => Promise<void>;
  selectInstance: (id: string) => void;
  updateInstanceStatus: (instanceId: string, newStatus: WorkflowInstanceStatus) => void;
  getInstanceDetails: (instanceId: string) => Promise<void>;
}

export const useWorkflowInstanceStore = create<WorkflowInstanceState>((set, get) => ({
  instances: [],
  selectedInstanceId: null,
  instanceDetails: null,
  isLoading: false,
  error: null,
  
  fetchInstances: async (filter) => {
    try {
      set({ isLoading: true, error: null });
      
      const instances = await fetchWorkflowInstances(filter);
      
      set({ 
        instances,
        isLoading: false 
      });
    } catch (error) {
      console.error('Failed to fetch workflow instances:', error);
      set({ 
        error: `Failed to fetch workflow instances: ${error}`,
        isLoading: false 
      });
    }
  },
  
  startInstance: async (workflowId, input) => {
    try {
      set({ isLoading: true, error: null });
      
      const instanceId = await startWorkflowInstance(workflowId, input);
      
      // Refresh the instances list
      await get().fetchInstances();
      
      set({ 
        selectedInstanceId: instanceId,
        isLoading: false 
      });
      
      return instanceId;
    } catch (error) {
      console.error('Failed to start workflow instance:', error);
      set({ 
        error: `Failed to start workflow instance: ${error}`,
        isLoading: false 
      });
      throw error;
    }
  },
  
  cancelInstance: async (instanceId) => {
    try {
      set({ isLoading: true, error: null });
      
      await cancelWorkflowInstance(instanceId);
      
      // Update the instance status locally
      set((state) => ({
        instances: state.instances.map(instance => 
          instance.instanceId === instanceId 
            ? { ...instance, status: 'CANCELLED' as WorkflowInstanceStatus } 
            : instance
        ),
        isLoading: false
      }));
    } catch (error) {
      console.error('Failed to cancel workflow instance:', error);
      set({ 
        error: `Failed to cancel workflow instance: ${error}`,
        isLoading: false 
      });
      throw error;
    }
  },
  
  selectInstance: (id) => {
    set((state) => ({ ...state, selectedInstanceId: id }));
  },
  
  updateInstanceStatus: (instanceId, newStatus) => {
    set((state) => {
      const existingIndex = state.instances.findIndex(i => i.instanceId === instanceId);
      
      if (existingIndex === -1) {
        // If the instance is not in our list, we might need to fetch it
        // For now, we'll just return the current state
        return state;
      }
      
      const updatedInstances = [...state.instances];
      updatedInstances[existingIndex] = {
        ...updatedInstances[existingIndex],
        status: newStatus,
        // If the status is terminal, set the end time
        ...(newStatus === 'COMPLETED' || newStatus === 'FAILED' || newStatus === 'CANCELLED' 
          ? { endTime: new Date().toISOString() } 
          : {})
      };
      
      // If we're viewing the details of this instance, update those too
      const updatedDetails = state.instanceDetails && state.instanceDetails.instanceId === instanceId
        ? { ...state.instanceDetails, status: newStatus }
        : state.instanceDetails;
      
      return {
        ...state,
        instances: updatedInstances,
        instanceDetails: updatedDetails
      };
    });
  },
  
  getInstanceDetails: async (instanceId) => {
    try {
      set({ isLoading: true, error: null });
      
      const details = await getWorkflowInstanceDetails(instanceId);
      
      set({ 
        instanceDetails: details,
        isLoading: false 
      });
    } catch (error) {
      console.error('Failed to get workflow instance details:', error);
      set({ 
        error: `Failed to get workflow instance details: ${error}`,
        isLoading: false 
      });
      throw error;
    }
  }
}));