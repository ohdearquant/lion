import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';

import { RuntimeStatus, LogEntry, WorkflowStatus } from '../types';
import { useRuntimeStatusStore } from '../stores/runtimeStatusStore';
import { useAgentStore } from '../stores/agentStore';
import { useLogStore } from '../stores/logStore';
import { useWorkflowStore } from '../stores/workflowStore';

/**
 * Runtime Status Event Listener
 */
export function useRuntimeStatusEvents() {
  useEffect(() => {
    let unlisten: () => void;

    const setupListener = async () => {
      // Get the updateStatus function from the store
      const updateStatus = useRuntimeStatusStore((state) => state.updateStatus);

      try {
        // Listen for runtime status changes
        unlisten = await listen<RuntimeStatus>('runtime_status_changed', (event) => {
          updateStatus(event.payload);
        });
      } catch (error) {
        console.error('Failed to set up runtime status listener:', error);
      }
    };

    setupListener();

    // Clean up the listener when the component unmounts
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);
}

/**
 * Agent Status Event Listener
 */
export function useAgentStatusEvents() {
  useEffect(() => {
    let unlisten: () => void;

    const setupListener = async () => {
      // Get the updateAgentStatus function from the store
      const updateAgentStatus = useAgentStore((state) => state.updateAgentStatus);

      try {
        // Listen for agent status changes
        unlisten = await listen<{ id: string, name: string, new_state: string }>('agent_status_changed', (event) => {
          const { id, new_state } = event.payload;
          updateAgentStatus(id, new_state);
        });
      } catch (error) {
        console.error('Failed to set up agent status listener:', error);
      }
    };

    setupListener();

    // Clean up the listener when the component unmounts
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);
}

/**
 * Log Event Listener
 */
export function useLogEvents() {
  useEffect(() => {
    let unlisten: () => void;

    const setupListener = async () => {
      // Get the addLogEntry function from the store
      const addLogEntry = useLogStore((state) => state.addLogEntry);

      try {
        // Listen for new log entries
        unlisten = await listen<LogEntry>('new_log_entry', (event) => {
          addLogEntry(event.payload);
        });
      } catch (error) {
        console.error('Failed to set up log listener:', error);
      }
    };

    setupListener();

    // Clean up the listener when the component unmounts
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);
}

/**
 * Workflow Status Event Listener
 */
export function useWorkflowStatusEvents() {
  useEffect(() => {
    let unlisten: () => void;

    const setupListener = async () => {
      // Get the updateInstanceStatus function from the store
      const updateInstanceStatus = useWorkflowStore((state) => state.updateInstanceStatus);

      try {
        // Listen for workflow status changes
        unlisten = await listen<{ instance_id: string, new_status: string, timestamp: string }>('workflow_status_changed', (event) => {
          const { instance_id, new_status } = event.payload;
          
          // Convert string status to enum
          let status: WorkflowStatus;
          switch (new_status.toLowerCase()) {
            case 'pending':
              status = WorkflowStatus.PENDING;
              break;
            case 'running':
              status = WorkflowStatus.RUNNING;
              break;
            case 'completed':
              status = WorkflowStatus.COMPLETED;
              break;
            case 'failed':
              status = WorkflowStatus.FAILED;
              break;
            case 'cancelled':
              status = WorkflowStatus.CANCELLED;
              break;
            default:
              console.warn(`Unknown workflow status: ${new_status}`);
              return;
          }
          
          updateInstanceStatus(instance_id, status);
        });
      } catch (error) {
        console.error('Failed to set up workflow status listener:', error);
      }
    };

    setupListener();

    // Clean up the listener when the component unmounts
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);
}

/**
 * Workflow Definition Loaded Event Listener
 */
export function useWorkflowDefinitionEvents() {
  useEffect(() => {
    let unlisten: () => void;

    const setupListener = async () => {
      // Get the loadDefinition function from the store
      const loadDefinition = useWorkflowStore((state) => state.loadDefinition);

      try {
        // Listen for workflow definition loaded events
        unlisten = await listen<{ workflow_id: string, name: string, file_path: string }>('workflow_definition_loaded', (event) => {
          const { file_path } = event.payload;
          loadDefinition(file_path).catch(error => {
            console.error('Failed to load workflow definition:', error);
          });
        });
      } catch (error) {
        console.error('Failed to set up workflow definition listener:', error);
      }
    };

    setupListener();

    // Clean up the listener when the component unmounts
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);
}

/**
 * Use all events
 * 
 * This is a convenience hook that sets up all event listeners
 */
export function useAllEvents() {
  useRuntimeStatusEvents();
  useAgentStatusEvents();
  useLogEvents();
  useWorkflowStatusEvents();
  useWorkflowDefinitionEvents();
}