/**
 * Runtime status information
 */
export interface RuntimeStatus {
  is_running: boolean;
  version: string;
  uptime_seconds: number;
  agent_count: number;
  error: string | null;
}

/**
 * Possible agent states
 */
export enum AgentState {
  STOPPED = "STOPPED",
  STARTING = "STARTING",
  RUNNING = "RUNNING",
  STOPPING = "STOPPING",
  ERROR = "ERROR"
}

/**
 * Agent information
 */
export interface Agent {
  id: string;
  name: string;
  agent_type: string;
  description: string;
  state: AgentState;
  capabilities: string[];
  version?: string;
}

/**
 * Log levels
 */
export enum LogLevel {
  DEBUG = "DEBUG",
  INFO = "INFO",
  WARNING = "WARNING",
  ERROR = "ERROR"
}

/**
 * Log entry
 */
export interface LogEntry {
  id: string;
  timestamp: string;
  level: LogLevel;
  source: string;
  message: string;
}

/**
 * Project information
 */
export interface Project {
  name: string;
  root_path: string;
  folders: string[];
  is_loaded: boolean;
}

/**
 * Workflow node in a graph
 */
export interface WorkflowNode {
  id: string;
  type: string;
  data: {
    label: string;
    [key: string]: any;
  };
  position: { x: number, y: number };
}

/**
 * Workflow edge in a graph
 */
export interface WorkflowEdge {
  id: string;
  source: string;
  target: string;
  label?: string;
}

/**
 * Workflow definition for UI display
 */
export interface WorkflowDefJson {
  id: string;
  name: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  properties: Record<string, any>;
}

/**
 * Workflow status
 */
export enum WorkflowStatus {
  PENDING = "PENDING",
  RUNNING = "RUNNING",
  COMPLETED = "COMPLETED",
  FAILED = "FAILED",
  CANCELLED = "CANCELLED"
}

/**
 * Workflow instance summary
 */
export interface WorkflowInstance {
  instanceId: string;
  workflowName: string;
  status: string;
  startTime: string;
  endTime?: string;
}

/**
 * Filter for listing workflow instances
 */
export interface InstanceFilter {
  status?: string;
  workflowName?: string;
  limit?: number;
}