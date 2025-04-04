import React, { useState, useEffect, useRef } from 'react';
import { useProjectStore } from '../../stores';
import { useUIStore } from '../../stores/uiStore';
import { useWorkflowStore } from '../../stores/workflowStore';
import { ContextMenu } from '../common/ContextMenu';

/**
 * Enhanced Project Explorer component with context menus
 * Purpose: Displays project folder structure in a tree view with context menu support
 * Props: None
 * State: 
 *   - Uses projectStore from Zustand
 *   - Uses uiStore for context menu management
 *   - Local state for expanded folders
 * API Calls: None (uses store data)
 */
export const EnhancedProjectExplorer: React.FC = () => {
  const project = useProjectStore(state => state.project);
  const { showContextMenu, hideContextMenu, contextMenu } = useUIStore();
  const { loadDefinition } = useWorkflowStore();
  
  const [expandedFolders, setExpandedFolders] = useState<Record<string, boolean>>({});
  const explorerRef = useRef<HTMLDivElement>(null);
  
  // Close context menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (explorerRef.current && !explorerRef.current.contains(event.target as Node)) {
        hideContextMenu();
      }
    };
    
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [hideContextMenu]);
  
  if (!project) {
    return null;
  }
  
  const toggleFolder = (folder: string) => {
    setExpandedFolders(prev => ({
      ...prev,
      [folder]: !prev[folder]
    }));
  };
  
  // Handle right-click on project root
  const handleProjectRootContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    
    const menuItems = [
      {
        label: 'New Workflow',
        action: () => {
          // This would open a dialog to create a new workflow
          console.log('Create new workflow');
        }
      },
      {
        label: 'Refresh Project',
        action: () => {
          console.log('Refresh project');
        }
      },
      {
        label: 'Close Project',
        action: () => {
          console.log('Close project');
        }
      }
    ];
    
    showContextMenu(e.clientX, e.clientY, 'project', project.root_path, menuItems);
  };
  
  // Handle right-click on folder
  const handleFolderContextMenu = (e: React.MouseEvent, folder: string) => {
    e.preventDefault();
    e.stopPropagation();
    
    const menuItems = [
      {
        label: 'New Workflow',
        action: () => {
          console.log(`Create new workflow in ${folder}`);
        }
      },
      {
        label: 'New Folder',
        action: () => {
          console.log(`Create new folder in ${folder}`);
        }
      },
      {
        label: `${expandedFolders[folder] ? 'Collapse' : 'Expand'} Folder`,
        action: () => toggleFolder(folder)
      }
    ];
    
    showContextMenu(e.clientX, e.clientY, 'folder', folder, menuItems);
  };
  
  // Handle right-click on file
  const handleFileContextMenu = (e: React.MouseEvent, file: string, path: string) => {
    e.preventDefault();
    
    const isWorkflow = file.endsWith('.workflow.json');
    
    const menuItems = [
      {
        label: isWorkflow ? 'Open Workflow' : 'Open File',
        action: () => {
          if (isWorkflow) {
            // Extract workflow ID from filename
            const workflowId = file.replace('.workflow.json', '');
            loadDefinition(path);
          } else {
            console.log(`Open file: ${path}`);
          }
        }
      },
      {
        label: 'Rename',
        action: () => {
          console.log(`Rename file: ${path}`);
        }
      },
      {
        label: 'Delete',
        action: () => {
          console.log(`Delete file: ${path}`);
        }
      }
    ];
    
    showContextMenu(e.clientX, e.clientY, 'file', path, menuItems);
  };
  
  // Mock files for demonstration
  const getMockFiles = (folder: string) => {
    if (folder === 'workflows') {
      return [
        { name: 'data-processing.workflow.json', type: 'workflow' },
        { name: 'notification-chain.workflow.json', type: 'workflow' },
        { name: 'README.md', type: 'file' }
      ];
    }
    
    if (folder === 'agents') {
      return [
        { name: 'data-processor.wasm', type: 'agent' },
        { name: 'notifier.wasm', type: 'agent' },
        { name: 'agent-config.json', type: 'file' }
      ];
    }
    
    // Default files for other folders
    return [
      { name: 'example.txt', type: 'file' },
      { name: 'config.json', type: 'file' }
    ];
  };
  
  // Get icon for file type
  const getFileIcon = (type: string) => {
    switch (type) {
      case 'workflow':
        return '🔄';
      case 'agent':
        return '🤖';
      default:
        return '📄';
    }
  };
  
  return (
    <div className="p-2" ref={explorerRef}>
      <h3 className="font-medium p-2 text-sm">Project Explorer</h3>
      
      <div className="text-sm">
        {/* Project root */}
        <div 
          className="flex items-center p-1 hover:bg-gray-300 dark:hover:bg-gray-700 rounded cursor-pointer"
          onContextMenu={handleProjectRootContextMenu}
        >
          <span className="mr-1">📁</span>
          <span className="font-medium">{project.name}</span>
        </div>
        
        {/* Standard folders */}
        <div className="ml-4">
          {project.folders.map(folder => (
            <div key={folder} className="my-1">
              <div 
                className="flex items-center p-1 hover:bg-gray-300 dark:hover:bg-gray-700 rounded cursor-pointer"
                onClick={() => toggleFolder(folder)}
                onContextMenu={(e) => handleFolderContextMenu(e, folder)}
              >
                <span className="mr-1">{expandedFolders[folder] ? '📂' : '📁'}</span>
                <span>{folder}</span>
              </div>
              
              {/* Folder contents */}
              {expandedFolders[folder] && (
                <div className="ml-4">
                  {getMockFiles(folder).map(file => (
                    <div 
                      key={file.name}
                      className="flex items-center p-1 hover:bg-gray-300 dark:hover:bg-gray-700 rounded cursor-pointer"
                      onContextMenu={(e) => handleFileContextMenu(e, file.name, `${project.root_path}/${folder}/${file.name}`)}
                      onClick={() => {
                        if (file.type === 'workflow') {
                          const workflowId = file.name.replace('.workflow.json', '');
                          loadDefinition(`${project.root_path}/${folder}/${file.name}`);
                        }
                      }}
                    >
                      <span className="mr-1">{getFileIcon(file.type)}</span>
                      <span>{file.name}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
      
      {/* Context menu is rendered by the ContextMenu component using the uiStore state */}
      {contextMenu.visible && (
        <ContextMenu
          visible={contextMenu.visible}
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenu.items}
          onClose={hideContextMenu}
        />
      )}
    </div>
  );
};