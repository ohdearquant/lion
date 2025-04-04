import React, { useCallback, useEffect, useState } from 'react';
import ReactFlow, {
  MiniMap,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  Edge,
  Node,
  ReactFlowProvider,
  NodeChange,
  EdgeChange,
  applyNodeChanges,
  applyEdgeChanges,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { useWorkflowDefinitionStore, WorkflowNode, WorkflowEdge } from '../../stores/workflowDefinitionStore';

interface WorkflowEditorProps {
  workflowId: string;
  readOnly?: boolean;
}

/**
 * Workflow Editor component using React Flow
 * Purpose: Visual editor for workflow definitions
 * Props:
 *   - workflowId: string - ID of the workflow definition to edit
 *   - readOnly: boolean - If true, the editor is in read-only mode
 * State:
 *   - Uses workflowDefinitionStore from Zustand
 *   - Uses React Flow hooks for nodes and edges
 * API Calls: None (uses store data, updates store on change)
 */
const WorkflowEditorComponent: React.FC<WorkflowEditorProps> = ({ workflowId, readOnly = false }) => {
  const { definitions, updateDefinition, isLoading, error } = useWorkflowDefinitionStore();
  const definition = definitions[workflowId];

  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  // Load nodes and edges from the definition
  useEffect(() => {
    if (definition) {
      const flowNodes: Node[] = definition.nodes.map((node: WorkflowNode) => ({
        id: node.id,
        position: node.position,
        type: node.type || 'default', // Ensure type is defined
        data: { label: node.data.label, ...node.data },
      }));
      const flowEdges: Edge[] = definition.edges.map((edge: WorkflowEdge) => ({
        id: edge.id,
        source: edge.source,
        target: edge.target,
        type: edge.type || 'default', // Ensure type is defined
        label: edge.label,
        animated: edge.animated,
      }));
      setNodes(flowNodes);
      setEdges(flowEdges);
    }
  }, [definition, setNodes, setEdges]);

  // Update the store when nodes or edges change (if not read-only)
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      if (!readOnly) {
        const updatedNodes = applyNodeChanges(changes, nodes);
        setNodes(updatedNodes);
        // Convert React Flow nodes back to WorkflowNode format for the store
        const storeNodes: WorkflowNode[] = updatedNodes.map(n => ({
          id: n.id,
          position: n.position,
          type: n.type,
          data: n.data,
        }));
        updateDefinition(workflowId, { nodes: storeNodes });
      }
    },
    [nodes, setNodes, updateDefinition, workflowId, readOnly]
  );

  const handleEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      if (!readOnly) {
        const updatedEdges = applyEdgeChanges(changes, edges);
        setEdges(updatedEdges);
        // Convert React Flow edges back to WorkflowEdge format for the store
        const storeEdges: WorkflowEdge[] = updatedEdges.map(e => ({
          id: e.id,
          source: e.source,
          target: e.target,
          type: e.type,
          label: e.label,
          animated: e.animated,
        }));
        updateDefinition(workflowId, { edges: storeEdges });
      }
    },
    [edges, setEdges, updateDefinition, workflowId, readOnly]
  );

  // Handle new connections (if not read-only)
  const onConnect = useCallback(
    (connection: Connection) => {
      if (!readOnly) {
        const newEdge = { ...connection, id: `edge-${Date.now()}`, type: 'default' };
        setEdges((eds) => addEdge(newEdge, eds));
        // Update store after adding edge
        const storeEdges: WorkflowEdge[] = [...edges, newEdge].map(e => ({
          id: e.id,
          source: e.source!, // Assert non-null as connection is valid
          target: e.target!, // Assert non-null as connection is valid
          type: e.type,
          label: e.label,
          animated: e.animated,
        }));
        updateDefinition(workflowId, { edges: storeEdges });
      }
    },
    [setEdges, edges, updateDefinition, workflowId, readOnly]
  );

  if (isLoading) {
    return (
      <div className="absolute inset-0 flex items-center justify-center bg-white bg-opacity-70 z-10">
        <div className="text-lg">Loading workflow...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="absolute inset-0 flex items-center justify-center bg-white bg-opacity-70 z-10">
        <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
          <p className="font-bold">Error</p>
          <p>{error}</p>
        </div>
      </div>
    );
  }

  if (!definition) {
    return (
      <div className="absolute inset-0 flex items-center justify-center bg-white bg-opacity-70 z-10">
        <div className="text-lg">Workflow definition not found.</div>
      </div>
    );
  }

  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={handleNodesChange}
        onEdgesChange={handleEdgesChange}
        onConnect={onConnect}
        fitView
        nodesDraggable={!readOnly}
        nodesConnectable={!readOnly}
        elementsSelectable={!readOnly}
        deleteKeyCode={readOnly ? null : 'Backspace'} // Disable delete in read-only
      >
        <MiniMap />
        <Controls />
        <Background />
        {/* Optional: Add a panel for workflow info */}
        <div className="absolute top-2 left-2 z-10">
          <div className="bg-white p-2 rounded shadow">
            <h3 className="font-medium text-sm mb-1">Workflow: {definition.name}</h3>
            {readOnly && <div className="text-xs text-gray-500">Read-only mode</div>}
          </div>
        </div>
      </ReactFlow>
    </div>
  );
};

// Wrap with ReactFlowProvider
export const WorkflowEditor: React.FC<WorkflowEditorProps> = (props) => (
  <ReactFlowProvider>
    <WorkflowEditorComponent {...props} />
  </ReactFlowProvider>
);