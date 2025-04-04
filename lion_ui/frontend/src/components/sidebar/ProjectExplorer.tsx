import React, { useState } from 'react';
import { useProjectStore } from '../../stores';

/**
 * Project explorer component
 * Purpose: Displays project folder structure in a tree view
 * Props: None
 * State: Uses projectStore from Zustand
 * API Calls: None (uses store data)
 * Event Listeners: None (store handles events)
 */
export const ProjectExplorer: React.FC = () => {
  const project = useProjectStore(state => state.project);
  const [expandedFolders, setExpandedFolders] = useState<Record<string, boolean>>({});
  
  if (!project) {
    return null;
  }
  
  const toggleFolder = (folder: string) => {
    setExpandedFolders(prev => ({
      ...prev,
      [folder]: !prev[folder]
    }));
  };
  
  return (
    <div className="p-2">
      <h3 className="font-medium p-2 text-sm">Project Explorer</h3>
      
      <div className="text-sm">
        {/* Project root */}
        <div className="flex items-center p-1 hover:bg-gray-300 dark:hover:bg-gray-700 rounded">
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
              >
                <span className="mr-1">{expandedFolders[folder] ? '📂' : '📁'}</span>
                <span>{folder}</span>
              </div>
              
              {/* Placeholder for folder contents (would be populated from actual file system) */}
              {expandedFolders[folder] && (
                <div className="ml-4">
                  <div className="flex items-center p-1 text-gray-600 dark:text-gray-400">
                    <span className="italic">Folder contents would be shown here</span>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};