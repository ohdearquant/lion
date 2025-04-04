import React, { useState, useEffect } from 'react';
import { useUIStore, TabInfo } from '../../stores/uiStore';
import { useProjectStore } from '../../stores/projectStore';
import { useWorkflowDefinitionStore } from '../../stores/workflowDefinitionStore';
import { useWorkflowInstanceStore } from '../../stores/workflowInstanceStore';
import { WorkflowEditor } from '../workflows/WorkflowEditor';
import { WorkflowInstancesView } from '../workflows/WorkflowInstancesView';
import { StartWorkflowModal } from '../workflows/StartWorkflowModal';
import { WelcomeScreen } from './WelcomeScreen'; // Assuming WelcomeScreen is exported correctly

/**
 * Enhanced Main Area component with Tabs
 * Purpose: Displays content based on active tab, manages tabs
 * Props: None
 * State:
 *   - Uses uiStore for tab management
 *   - Uses projectStore, workflowDefinitionStore, workflowInstanceStore for data
 *   - Local state for StartWorkflowModal visibility and target workflow
 * API Calls: None (uses store data)
 */
export const EnhancedMainArea: React.FC = () => {
  const { tabs, activeTabId, setActiveTab, closeTab, addTab } = useUIStore();
  const { project } = useProjectStore();
  const { definitions } = useWorkflowDefinitionStore();
  const { instances, selectInstance } = useWorkflowInstanceStore();
  
  const [isStartModalOpen, setIsStartModalOpen] = useState(false);
  const [workflowToStart, setWorkflowToStart] = useState<string | null>(null);
  
  // Function to open a workflow editor tab
  const openWorkflowEditor = (workflowId: string) => {
    const definition = definitions[workflowId];
    const tabId = `workflow-editor-${workflowId}`;
    const existingTab = tabs.find(
      (tab) => tab.type === 'workflow' && tab.id === tabId
    );
    
    if (!existingTab) {
      addTab({
        id: tabId,
        title: definition?.name || `Workflow: ${workflowId.substring(0, 8)}...`,
        type: 'workflow',
        content: { workflowId } // Pass workflowId in content
      });
    } else {
      setActiveTab(tabId);
    }
  };
  
  // Function to open the workflow instances tab
  const openWorkflowInstances = () => {
    const tabId = 'workflow-instances';
    const existingTab = tabs.find((tab) => tab.id === tabId);
    
    if (!existingTab) {
      addTab({
        id: tabId,
        title: 'Workflow Instances',
        type: 'instances',
      });
    } else {
      setActiveTab(tabId);
    }
  };
  
  // Function to open a workflow instance details tab (placeholder)
  const openInstanceDetails = (instanceId: string) => {
    const instance = instances.find((inst) => inst.instanceId === instanceId);
    const tabId = `instance-details-${instanceId}`;
    const existingTab = tabs.find(
      (tab) => tab.type === 'instances' && tab.id === tabId // Assuming 'instances' type for details too for now
    );
    
    if (!existingTab) {
      addTab({
        id: tabId,
        title: `Instance: ${instance?.workflowName || instanceId.substring(0, 8)}...`,
        type: 'instances', // Re-use instances view for now, or create a new type/component
        content: { instanceId } // Pass instanceId in content
      });
    } else {
      setActiveTab(tabId);
    }
    // Select the instance in the store as well
    selectInstance(instanceId);
  };
  
  // Function to open the Start Workflow modal
  const handleOpenStartModal = (workflowId: string) => {
    setWorkflowToStart(workflowId);
    setIsStartModalOpen(true);
  };
  
  // Function to handle successful workflow start
  const handleStartSuccess = (instanceId: string) => {
    setIsStartModalOpen(false);
    setWorkflowToStart(null);
    // Optionally open the instance details tab or refresh instances view
    openWorkflowInstances(); // Go to instances view after starting
    setActiveTab('workflow-instances');
  };
  
  // Render the content of the active tab
  const renderTabContent = () => {
    const activeTab = tabs.find((tab) => tab.id === activeTabId);
    
    if (!activeTab) {
      // Default to Welcome screen if no active tab or tab not found
      return <WelcomeScreen />;
    }
    
    switch (activeTab.type) {
      case 'welcome':
        return <WelcomeScreen />;
      case 'project':
        // Placeholder for Project Overview component
        return (
          <div className="p-4">
            <h2 className="text-xl font-semibold">Project Overview: {project?.name}</h2>
            <p>Details about the project would go here.</p>
            {/* Add buttons or links relevant to the project */}
            <button 
              onClick={openWorkflowInstances}
              className="mt-4 px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
            >
              View Workflow Instances
            </button>
          </div>
        );
      case 'workflow':
        // Render WorkflowEditor, passing workflowId from tab content
        const workflowId = activeTab.content?.workflowId;
        if (!workflowId) return <div>Error: Workflow ID missing in tab data.</div>;
        return (
          <div className="h-full flex flex-col">
            <div className="p-2 bg-gray-100 dark:bg-gray-700 border-b border-gray-300 dark:border-gray-600 flex items-center">
              <button 
                className="px-2 py-1 text-sm bg-green-500 text-white rounded hover:bg-green-600 mr-2"
                onClick={() => handleOpenStartModal(workflowId)}
              >
                Start Workflow
              </button>
              <button 
                className="px-2 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600"
                // Add save functionality here, likely calling saveDefinition from the store
                onClick={() => console.log(`Save workflow ${workflowId}`)} 
              >
                Save
              </button>
            </div>
            <div className="flex-1">
              <WorkflowEditor workflowId={workflowId} />
            </div>
          </div>
        );
      case 'instances':
        // Render WorkflowInstancesView
        // If instanceId is present in content, it implies detail view (though handled by the component itself for now)
        return (
          <div className="h-full">
            <WorkflowInstancesView onSelectInstance={openInstanceDetails} />
          </div>
        );
      default:
        return <div>Unknown tab type: {activeTab.type}</div>;
    }
  };
  
  // If no project is open, show the Welcome Screen
  if (!project && tabs.length === 1 && tabs[0].id === 'welcome') {
    return <WelcomeScreen />;
  }
  
  // If project is open but no tabs (shouldn't happen with default welcome), show welcome
  if (tabs.length === 0) {
    return <WelcomeScreen />;
  }
  
  return (
    <div className="flex flex-col h-full">
      {/* Tab Bar */}
      {tabs.length > 0 && (
        <div className="flex bg-gray-200 dark:bg-gray-800 border-b border-gray-300 dark:border-gray-700">
          {tabs.map((tab) => (
            <div
              key={tab.id}
              className={`flex items-center px-4 py-2 cursor-pointer border-r border-gray-300 dark:border-gray-700 ${
                activeTabId === tab.id
                  ? 'bg-white dark:bg-gray-900 border-b-2 border-blue-500'
                  : 'bg-gray-200 dark:bg-gray-800 hover:bg-gray-300 dark:hover:bg-gray-700'
              }`}
              onClick={() => setActiveTab(tab.id)}
            >
              <span className="truncate max-w-xs">{tab.title}</span>
              <button
                className="ml-2 text-gray-500 hover:text-gray-800 dark:hover:text-gray-200 text-xs"
                onClick={(e) => {
                  e.stopPropagation(); // Prevent tab activation when closing
                  closeTab(tab.id);
                }}
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}
      
      {/* Tab Content */}
      <div className="flex-1 overflow-auto bg-white dark:bg-gray-900">
        {renderTabContent()}
      </div>
      
      {/* Start Workflow Modal */}
      {workflowToStart && (
        <StartWorkflowModal
          isOpen={isStartModalOpen}
          workflowId={workflowToStart}
          onClose={() => setIsStartModalOpen(false)}
          onSuccess={handleStartSuccess}
        />
      )}
    </div>
  );
};