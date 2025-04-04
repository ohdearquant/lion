import React from 'react';
import { WelcomeScreen } from './WelcomeScreen';
import { useProjectStore } from '../../stores';

/**
 * Main content area component
 * Purpose: Container for the main content area (welcome screen initially)
 * Props: None
 * State: None (container only)
 * Children: WelcomeScreen or other content based on application state
 */
export const MainArea: React.FC = () => {
  const project = useProjectStore(state => state.project);
  
  return (
    <div className="flex-1 overflow-auto bg-white dark:bg-gray-900">
      {!project ? (
        <WelcomeScreen />
      ) : (
        <div className="p-6">
          <h1 className="text-2xl font-bold mb-4">{project.name}</h1>
          
          <div className="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg mb-6">
            <h2 className="text-lg font-semibold mb-2">Project Information</h2>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">Location:</p>
                <p className="font-mono text-sm">{project.root_path}</p>
              </div>
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-400">Status:</p>
                <p className="text-sm">
                  {project.is_loaded ? (
                    <span className="text-green-600 dark:text-green-400">Loaded</span>
                  ) : (
                    <span className="text-yellow-600 dark:text-yellow-400">Not Loaded</span>
                  )}
                </p>
              </div>
            </div>
          </div>
          
          <div className="grid grid-cols-2 gap-6">
            <div className="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg">
              <h2 className="text-lg font-semibold mb-2">Project Structure</h2>
              <ul className="text-sm">
                {project.folders.map(folder => (
                  <li key={folder} className="mb-1 flex items-center">
                    <span className="mr-2">📁</span>
                    {folder}
                  </li>
                ))}
              </ul>
            </div>
            
            <div className="bg-gray-100 dark:bg-gray-800 p-4 rounded-lg">
              <h2 className="text-lg font-semibold mb-2">Quick Actions</h2>
              <div className="space-y-2">
                <button className="w-full px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 text-sm">
                  View Agents
                </button>
                <button className="w-full px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 text-sm">
                  View Logs
                </button>
                <button className="w-full px-4 py-2 bg-gray-300 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded hover:bg-gray-400 dark:hover:bg-gray-600 text-sm">
                  Close Project
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};