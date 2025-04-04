import React, { useState } from 'react';
import { useWorkflowInstanceStore } from '../../stores/workflowInstanceStore';
import { useWorkflowDefinitionStore } from '../../stores/workflowDefinitionStore';

/**
 * StartWorkflowModal component
 * Purpose: Modal dialog for starting a workflow with input data
 * Props:
 *   - workflowId: string - ID of the workflow to start
 *   - isOpen: boolean - Whether the modal is open
 *   - onClose: () => void - Function to call when the modal is closed
 *   - onSuccess: (instanceId: string) => void - Function to call when the workflow is started successfully
 * State:
 *   - inputData: string - JSON input data for the workflow
 *   - isSubmitting: boolean - Whether the form is being submitted
 *   - error: string | null - Error message if submission fails
 */
export const StartWorkflowModal: React.FC<{
  workflowId: string;
  isOpen: boolean;
  onClose: () => void;
  onSuccess: (instanceId: string) => void;
}> = ({ workflowId, isOpen, onClose, onSuccess }) => {
  const [inputData, setInputData] = useState('{\n  "key": "value"\n}');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const { startInstance } = useWorkflowInstanceStore();
  const { definitions } = useWorkflowDefinitionStore();
  
  const workflowName = definitions[workflowId]?.name || workflowId;
  
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!inputData.trim()) {
      setError('Input data cannot be empty');
      return;
    }
    
    try {
      // Validate JSON
      JSON.parse(inputData);
      
      setIsSubmitting(true);
      setError(null);
      
      // Start the workflow instance
      const instanceId = await startInstance(workflowId, inputData);
      
      // Call the success callback
      onSuccess(instanceId);
      
      // Close the modal
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start workflow');
      setIsSubmitting(false);
    }
  };
  
  if (!isOpen) {
    return null;
  }
  
  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg w-full max-w-md">
        <div className="p-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-lg font-semibold">
            Start Workflow: {workflowName}
          </h2>
        </div>
        
        <form onSubmit={handleSubmit}>
          <div className="p-4">
            <div className="mb-4">
              <label className="block text-sm font-medium mb-1">
                Input Data (JSON)
              </label>
              <textarea
                className="w-full h-40 p-2 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-mono text-sm"
                value={inputData}
                onChange={(e) => setInputData(e.target.value)}
                disabled={isSubmitting}
              />
              <p className="text-xs text-gray-500 mt-1">
                Enter the input data for this workflow in JSON format.
              </p>
            </div>
            
            {error && (
              <div className="mb-4 p-2 bg-red-100 border border-red-400 text-red-700 rounded">
                {error}
              </div>
            )}
          </div>
          
          <div className="p-4 bg-gray-100 dark:bg-gray-700 flex justify-end space-x-2 rounded-b-lg">
            <button
              type="button"
              className="px-4 py-2 bg-gray-300 dark:bg-gray-600 text-gray-800 dark:text-gray-200 rounded hover:bg-gray-400 dark:hover:bg-gray-500"
              onClick={onClose}
              disabled={isSubmitting}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
              disabled={isSubmitting}
            >
              {isSubmitting ? 'Starting...' : 'Start Workflow'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};