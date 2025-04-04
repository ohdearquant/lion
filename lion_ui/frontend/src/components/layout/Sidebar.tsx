import React from 'react';
import { ProjectExplorer } from '../sidebar/ProjectExplorer';
import { useProjectStore } from '../../stores';

/**
 * Sidebar component
 * Purpose: Container for sidebar elements (project explorer, etc.)
 * Props: None
 * State: None (container only)
 * Children: ProjectExplorer
 */
export const Sidebar: React.FC = () => {
  const project = useProjectStore(state => state.project);
  const selectAndOpenProject = useProjectStore(state => state.selectAndOpenProject);
  
  return (
    <div className="w-64 h-full bg-gray-200 dark:bg-gray-800 border-r border-gray-300 dark:border-gray-700 flex flex-col">
      <div className="p-4 border-b border-gray-300 dark:border-gray-700">
        <h2 className="text-lg font-semibold">LionForge IDE</h2>
      </div>
      
      {/* Project section */}
      <div className="p-4 border-b border-gray-300 dark:border-gray-700">
        <div className="flex justify-between items-center mb-2">
          <h3 className="font-medium">Project</h3>
          <button 
            onClick={() => selectAndOpenProject()}
            className="text-sm px-2 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
          >
            Open
          </button>
        </div>
        
        {project && (
          <div className="text-sm text-gray-600 dark:text-gray-400">
            <p className="truncate">{project.name}</p>
            <p className="truncate text-xs">{project.root_path}</p>
          </div>
        )}
      </div>
      
      {/* Project Explorer */}
      <div className="flex-1 overflow-auto">
        {project ? (
          <ProjectExplorer />
        ) : (
          <div className="p-4 text-sm text-gray-500 dark:text-gray-400">
            No project open. Click "Open" to select a project.
          </div>
        )}
      </div>
    </div>
  );
};