import { invoke } from '@tauri-apps/api/tauri';
import { open } from '@tauri-apps/api/dialog';

import { RuntimeStatus, Agent, LogEntry, WorkflowDefJson, WorkflowInstance } from '../types';

/**
 * Runtime Status API
 */
export async function fetchRuntimeStatus(): Promise<RuntimeStatus> {
  return invoke<RuntimeStatus>('get_runtime_status');
}

/**
 * Agent API
 */
export async function fetchAgents(): Promise<Agent[]> {
  return invoke<Agent[]>('list_agents');
}

export async function loadAgent(path: string): Promise<Agent> {
  return invoke<Agent>('load_agent', { path });
}

export async function unloadAgent(id: string): Promise<boolean> {
  return invoke<boolean>('unload_agent', { id });
}

/**
 * Log API
 */
export async function fetchRecentLogs(limit: number = 100): Promise<LogEntry[]> {
  return invoke<LogEntry[]>('get_recent_logs', { limit });
}

export async function clearLogs(): Promise<boolean> {
  return invoke<boolean>('clear_logs');
}

/**
 * Project API
 */
export async function openProject(): Promise<{ name: string; root_path: string; folders: string[] }> {
  // Open a folder selection dialog
  const selected = await open({
    directory: true,
    multiple: false,
    title: 'Select Project Folder'
  });

  if (selected === null) {
    throw new Error('No folder selected');
  }

  const path = Array.isArray(selected) ? selected[0] : selected;
  
  // Call the Tauri command to open the project
  return invoke('open_project', { path });
}

/**
 * Workflow API
 */
export async function fetchWorkflowDefinitions(): Promise<Record<string, WorkflowDefJson>> {
  return invoke<Record<string, WorkflowDefJson>>('list_workflow_definitions');
}

export async function loadWorkflowDefinition(path: string): Promise<WorkflowDefJson> {
  return invoke<WorkflowDefJson>('load_workflow_definition', { path });
}

export async function fetchWorkflowInstances(statusFilter?: string): Promise<WorkflowInstance[]> {
  return invoke<WorkflowInstance[]>('list_workflow_instances', { statusFilter });
}

export async function startWorkflowInstance(
  workflowId: string, 
  inputData: string
): Promise<WorkflowInstance> {
  return invoke<WorkflowInstance>('start_workflow_instance', { 
    workflowId, 
    inputData 
  });
}

export async function cancelWorkflowInstance(instanceId: string): Promise<boolean> {
  return invoke<boolean>('cancel_workflow_instance', { instanceId });
}

export async function getWorkflowInstanceDetails(instanceId: string): Promise<WorkflowInstance> {
  return invoke<WorkflowInstance>('get_workflow_instance_details', { instanceId });
}

/**
 * File System API
 */
export async function selectFile(options: {
  title?: string;
  filters?: { name: string; extensions: string[] }[];
}): Promise<string | null> {
  const selected = await open({
    multiple: false,
    title: options.title || 'Select File',
    filters: options.filters
  });

  if (selected === null) {
    return null;
  }

  return Array.isArray(selected) ? selected[0] : selected;
}

export async function selectWorkflowFile(): Promise<string | null> {
  return selectFile({
    title: 'Select Workflow Definition File',
    filters: [{ name: 'Workflow Files', extensions: ['json', 'yaml', 'yml'] }]
  });
}

export async function selectAgentFile(): Promise<string | null> {
  return selectFile({
    title: 'Select Agent File',
    filters: [{ name: 'WebAssembly Files', extensions: ['wasm'] }]
  });
}