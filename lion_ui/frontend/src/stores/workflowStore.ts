import { create } from 'zustand';
import { 
  WorkflowDefJson, 
  WorkflowInstance, 
  WorkflowStatus 
} from '../types';
import { 
  loadWorkflowDefinition, 
  startWorkflowInstance, 
  listWorkflowInstances, 
  cancelWorkflowInstance,
  getWorkflowInstance
} from '../lib/api';

/**
 * Workflow Store State
 */
interface WorkflowStore {
  // Workflow Definitions
  definitions: Record<string, WorkflowDefJson>;
  definitionsLoading: boolean;
  definitionsError: string | null;
  selectedDefinitionId: string | null;
  
  // Workflow Instances
  instances: WorkflowInstance[];
  instancesLoading: boolean;
  instancesError: string | null;
  selectedInstanceId: string | null;
  
  // Actions - Definitions
  loadDefinition: (path: string) => Promise<WorkflowDefJson>;
  selectDefinition: (id: string) => void;
  
  // Actions - Instances
  fetchInstances: (filter?: string) => Promise<WorkflowInstance[]>;
  startInstance: (workflowId: string, inputData: string) => Promise<WorkflowInstance>;
  cancelInstance: (instanceId: string) => Promise<void>;
  getInstanceDetails: (instanceId: string) => Promise<WorkflowInstance>;
  updateInstanceStatus: (instanceId: string, newStatus: WorkflowStatus) => void;
  selectInstance: (id: string) => void;
}

/**
 * Workflow Store Implementation
 */
export const useWorkflowStore = create<WorkflowStore>((set, get) => ({
  // Initial state - Definitions
  definitions: {},
  definitionsLoading: false,
  definitionsError: null,
  selectedDefinitionId: null,
  
  // Initial state - Instances
  instances: [],
  instancesLoading: false,
  instancesError: null,
  selectedInstanceId: null,
  
  // Actions - Definitions
  loadDefinition: async (path: string) => {
    try {
      set((state) => ({ 
        ...state, 
        definitionsLoading: true,
        definitionsError: null
      }));
      
      const definition = await loadWorkflowDefinition(path);
      
      set((state) => ({ 
        ...state, 
        definitions: {
          ...state.definitions,
          [definition.id]: definition
        },
        definitionsLoading: false
      }));
      
      return definition;
    } catch (error) {
      set((state) => ({ 
        ...state, 
        definitionsLoading: false,
        definitionsError: error instanceof Error ? error.message : String(error)
      }));
      throw error;
    }
  },
  
  selectDefinition: (id: string) => {
    set((state) => ({ ...state, selectedDefinitionId: id }));
  },
  
  // Actions - Instances
  fetchInstances: async (filter) => {
    try {
      set((state) => ({ 
        ...state, 
        instancesLoading: true,
        instancesError: null
      }));
      
      const instances = await listWorkflowInstances(filter);
      
      set((state) => ({ 
        ...state, 
        instances,
        instancesLoading: false
      }));
      
      return instances;
    } catch (error) {
      set((state) => ({ 
        ...state, 
        instancesLoading: false,
        instancesError: error instanceof Error ? error.message : String(error)
      }));
      throw error;
    }
  },
  
  startInstance: async (workflowId, inputData) => {
    try {
      set((state) => ({ 
        ...state, 
        instancesLoading: true,
        instancesError: null
      }));
      
      const instance = await startWorkflowInstance(workflowId, inputData);
      
      set((state) => ({ 
        ...state, 
        instances: [...state.instances, instance],
        instancesLoading: false
      }));
      
      return instance;
    } catch (error) {
      set((state) => ({ 
        ...state, 
        instancesLoading: false,
        instancesError: error instanceof Error ? error.message : String(error)
      }));
      throw error;
    }
  },
  
  cancelInstance: async (instanceId) => {
    try {
      set((state) => ({ 
        ...state, 
        instancesLoading: true,
        instancesError: null
      }));
      
      await cancelWorkflowInstance(instanceId);
      
      // Update the instance status in the store
      set((state) => ({ 
        ...state, 
        instances: state.instances.map(instance => 
          instance.instanceId === instanceId 
            ? { ...instance, status: 'CANCELLED' as WorkflowStatus } 
            : instance
        ),
        instancesLoading: false
      }));
    } catch (error) {
      set((state) => ({ 
        ...state, 
        instancesLoading: false,
        instancesError: error instanceof Error ? error.message : String(error)
      }));
      throw error;
    }
  },
  
  updateInstanceStatus: (instanceId, newStatus) => {
    set((state) => {
      const existingIndex = state.instances.findIndex(i => i.instanceId === instanceId);
      
      if (existingIndex === -1) {
        return state;
      }
      
      const updatedInstances = [...state.instances];
      const instance = updatedInstances[existingIndex];
      
      updatedInstances[existingIndex] = {
        ...instance,
        status: newStatus,
        // If the status is completed, failed, or cancelled, set the end time
        ...(newStatus === 'COMPLETED' || 
           newStatus === 'FAILED' || 
           newStatus === 'CANCELLED'
          ? { endTime: new Date().toISOString() }
          : {})
      };
      
      return {
        ...state,
        instances: updatedInstances
      };
    });
  },
  
  selectInstance: (id: string) => {
    set((state) => ({ ...state, selectedInstanceId: id }));
  },
  
  getInstanceDetails: async (instanceId) => {
    try {
      set((state) => ({ 
        ...state, 
        instancesLoading: true,
        instancesError: null
      }));
      
      const instance = await getWorkflowInstance(instanceId);
      
      // Update the instance in the store
      set((state) => {
        const existingIndex = state.instances.findIndex(i => i.instanceId === instanceId);
        
        let updatedInstances = [...state.instances];
        
        if (existingIndex !== -1) {
          // Update existing instance
          updatedInstances[existingIndex] = instance;
        } else {
          // Add new instance
          updatedInstances = [...updatedInstances, instance];
        }
        
        return {
          ...state,
          instances: updatedInstances,
          instancesLoading: false
        };
      });
      
      return instance;
    } catch (error) {
      set((state) => ({ 
        ...state, 
        instancesLoading: false,
        instancesError: error instanceof Error ? error.message : String(error)
      }));
      throw error;
    }
  }
}));