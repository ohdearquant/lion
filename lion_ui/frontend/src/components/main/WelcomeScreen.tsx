import React from 'react';
import { useProjectStore } from '../../stores';

/**
 * Welcome screen component
 * Purpose: Initial screen shown when no project is open
 * Props: None
 * State: None
 * API Calls: openProject() when "Open Project" button is clicked
 */
export const WelcomeScreen: React.FC = () => {
  const selectAndOpenProject = useProjectStore(state => state.selectAndOpenProject);
  
  return (
    <div className="flex flex-col items-center justify-center h-full p-8">
      <div className="text-center max-w-2xl">
        <h1 className="text-4xl font-bold mb-6">Welcome to LionForge IDE</h1>
        
        <p className="text-lg text-gray-600 dark:text-gray-400 mb-8">
          LionForge IDE is a development environment for building and managing Lion runtime applications.
          Get started by opening a project or creating a new one.
        </p>
        
        <div className="flex justify-center space-x-4">
          <button
            onClick={() => selectAndOpenProject()}
            className="px-6 py-3 bg-blue-500 text-white rounded-lg hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50"
          >
            Open Project
          </button>
          
          <button
            className="px-6 py-3 bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-opacity-50"
          >
            Create New Project
          </button>
        </div>
        
        <div className="mt-12 grid grid-cols-2 gap-6">
          <div className="bg-gray-100 dark:bg-gray-800 p-6 rounded-lg text-left">
            <h2 className="text-xl font-semibold mb-3">Recent Projects</h2>
            <p className="text-gray-600 dark:text-gray-400 text-sm mb-4">
              No recent projects found.
            </p>
          </div>
          
          <div className="bg-gray-100 dark:bg-gray-800 p-6 rounded-lg text-left">
            <h2 className="text-xl font-semibold mb-3">Quick Links</h2>
            <ul className="space-y-2 text-blue-500 dark:text-blue-400">
              <li>
                <a href="#" className="hover:underline">Documentation</a>
              </li>
              <li>
                <a href="#" className="hover:underline">Examples</a>
              </li>
              <li>
                <a href="#" className="hover:underline">Report an Issue</a>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
};