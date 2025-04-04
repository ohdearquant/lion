/**
 * Runtime Status
 */
export interface RuntimeStatus {
  state: 'running' | 'stopped' | 'error';
  uptime_seconds: number;
  version: string;
  plugins_loaded: number;
  agents_active: number;
  error?: string;
}

/**
 * Agent Types
 */
export enum AgentState {
  STOPPED = 'stopped',
  STARTING = 'starting',
  RUNNING = 'running',
  STOPPING = 'stopping',
  ERROR = 'error'
}

export interface Agent {
  id: string;
  name: string;
  description?: string;
  version?: string;
  state: AgentState;
  capabilities: string[];
  path: string;
  error?: string;
  metadata?: Record<string, any>;
}

/**
 * Log Types
 */
export enum LogLevel {
  DEBUG = 'debug',
  INFO = 'info',
  WARN = 'warn',
  ERROR = 'error'
}

export interface LogEntry {
  id: string;
  timestamp: string;
  level: LogLevel;
  source: string;
  message: string;
  metadata?: Record<string, any>;
}

/**
 * Project Types
 */
export interface Project {
  name: string;
  root_path: string;
  description?: string;
  folders: string[];
  is_loaded: boolean;
}

/**
 * Workflow Types
 */
export enum WorkflowStatus {
  PENDING = 'pending',
  RUNNING = 'running',
  COMPLETED = 'completed',
  FAILED = 'failed',
  CANCELLED = 'cancelled'
}

export interface WorkflowNode {
  id: string;
  type?: string;
  position: [number, number];
  data: {
    label: string;
    [key: string]: any;
  };
}

export interface WorkflowEdge {
  id: string;
  source: string;
  target: string;
  sourceHandle?: string;
  targetHandle?: string;
  label?: string;
}

export interface WorkflowDefJson {
  id: string;
  name: string;
  description?: string;
  version?: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  metadata?: Record<string, any>;
}

export interface WorkflowInstance {
  instanceId: string;
  workflowId: string;
  workflowName: string;
  status: WorkflowStatus;
  startTime: string;
  endTime?: string;
  inputData?: string;
  outputData?: string;
  error?: string;
  progress?: number;
  currentNodeId?: string;
}

/**
 * UI Types
 */
export interface TabData {
  id: string;
  type: 'workflow-editor' | 'workflow-instances' | 'workflow-instance-details';
  title: string;
  data: {
    workflowId?: string;
    instanceId?: string;
    [key: string]: any;
  };
}

export interface ContextMenuItem {
  id: string;
  label: string;
  icon?: string;
  action: () => void;
  disabled?: boolean;
  separator?: boolean;
}

export interface ContextMenu {
  visible: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
}