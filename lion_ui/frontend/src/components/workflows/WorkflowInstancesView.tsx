import React, { useEffect, useState } from 'react';
import { useWorkflowInstanceStore, WorkflowInstanceStatus } from '../../stores/workflowInstanceStore';
import { formatDistanceToNow } from 'date-fns';

interface WorkflowInstancesViewProps {
  onSelectInstance: (instanceId: string) => void;
}

/**
 * Workflow Instances View component
 * Purpose: Displays a list of workflow instances with filtering and actions
 * Props:
 *   - onSelectInstance: (instanceId: string) => void - Callback when an instance is selected
 * State:
 *   - Uses workflowInstanceStore from Zustand
 *   - Local state for status filter
 * API Calls:
 *   - fetchInstances
 *   - cancelInstance
 */
export const WorkflowInstancesView: React.FC<WorkflowInstancesViewProps> = ({ onSelectInstance }) => {
  const { 
    instances, 
    fetchInstances, 
    cancelInstance, 
    isLoading, 
    error, 
    selectedInstanceId,
    selectInstance
  } = useWorkflowInstanceStore();
  
  const [statusFilter, setStatusFilter] = useState<WorkflowInstanceStatus | null>(null);
  
  // Fetch instances on mount and when filter changes
  useEffect(() => {
    fetchInstances({ status: statusFilter || undefined });
  }, [fetchInstances, statusFilter]);
  
  // Handle instance cancellation
  const handleCancel = async (instanceId: string) => {
    if (window.confirm('Are you sure you want to cancel this workflow instance?')) {
      try {
        await cancelInstance(instanceId);
      } catch (err) {
        console.error('Failed to cancel instance:', err);
        // Optionally show an error message to the user
      }
    }
  };
  
  // Format date for display
  const formatDate = (dateString?: string) => {
    if (!dateString) return '-';
    try {
      return formatDistanceToNow(new Date(dateString), { addSuffix: true });
    } catch (e) {
      return 'Invalid Date';
    }
  };
  
  // Get color based on status
  const getStatusColor = (status: WorkflowInstanceStatus) => {
    switch (status) {
      case 'RUNNING': return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300';
      case 'COMPLETED': return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300';
      case 'FAILED': return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300';
      case 'CANCELLED': return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300';
      case 'PENDING': return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300';
      default: return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300';
    }
  };
  
  return (
    <div className="h-full flex flex-col">
      {/* Toolbar */}
      <div className="flex items-center p-2 bg-gray-100 dark:bg-gray-700 border-b border-gray-300 dark:border-gray-600">
        <button 
          className="px-2 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600"
          onClick={() => fetchInstances({ status: statusFilter || undefined })}
          disabled={isLoading}
        >
          Refresh
        </button>
        
        <div className="flex items-center space-x-2 ml-4">
          <span className="text-xs">Status:</span>
          <select 
            className="text-xs p-1 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
            value={statusFilter || ''}
            onChange={(e) => setStatusFilter(e.target.value as WorkflowInstanceStatus || null)}
          >
            <option value="">All Statuses</option>
            <option value="PENDING">Pending</option>
            <option value="RUNNING">Running</option>
            <option value="COMPLETED">Completed</option>
            <option value="FAILED">Failed</option>
            <option value="CANCELLED">Cancelled</option>
          </select>
        </div>
        
        <div className="flex-1"></div> {/* Spacer */}
        
        {isLoading && <span className="text-xs text-gray-600 dark:text-gray-400">Loading...</span>}
        {error && <span className="text-xs text-red-600 dark:text-red-400">{error}</span>}
      </div>
      
      {/* Instance List */}
      <div className="flex-1 overflow-auto">
        {instances.length === 0 && !isLoading && (
          <div className="text-gray-500 dark:text-gray-400 p-4 text-center">
            No workflow instances found.
          </div>
        )}
        {instances.length > 0 && (
          <table className="min-w-full divide-y divide-gray-300 dark:divide-gray-700">
            <thead className="bg-gray-100 dark:bg-gray-800">
              <tr>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Instance ID
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Workflow
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Status
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Start Time
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  End Time
                </th>
                <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="bg-white dark:bg-gray-900 divide-y divide-gray-200 dark:divide-gray-800">
              {instances.map(instance => (
                <tr 
                  key={instance.instanceId} 
                  className={`hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer ${selectedInstanceId === instance.instanceId ? 'bg-blue-50 dark:bg-blue-900' : ''}`}
                  onClick={() => {
                    selectInstance(instance.instanceId);
                    onSelectInstance(instance.instanceId);
                  }}
                >
                  <td className="px-4 py-2 text-sm font-mono text-gray-600 dark:text-gray-400">
                    {instance.instanceId.substring(0, 8)}...
                  </td>
                  <td className="px-4 py-2 text-sm">
                    {instance.workflowName}
                  </td>
                  <td className="px-4 py-2 text-sm">
                    <span className={`px-2 py-1 rounded-full text-xs ${getStatusColor(instance.status)}`}>
                      {instance.status}
                    </span>
                  </td>
                  <td className="px-4 py-2 text-sm">
                    {formatDate(instance.startTime)}
                  </td>
                  <td className="px-4 py-2 text-sm">
                    {formatDate(instance.endTime)}
                  </td>
                  <td className="px-4 py-2 text-sm">
                    {instance.status === 'RUNNING' || instance.status === 'PENDING' ? (
                      <button 
                        className="text-red-500 hover:text-red-700 text-xs"
                        onClick={(e) => {
                          e.stopPropagation(); // Prevent row click
                          handleCancel(instance.instanceId);
                        }}
                      >
                        Cancel
                      </button>
                    ) : (
                      <span className="text-gray-400">-</span>
                    )}
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