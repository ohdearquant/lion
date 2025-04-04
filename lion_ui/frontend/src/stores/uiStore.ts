import { create } from 'zustand';

/**
 * Tab information
 */
export interface TabInfo {
  id: string;
  title: string;
  type: 'workflow' | 'welcome' | 'project' | 'instances';
  path?: string;
  content?: any;
}

/**
 * Context menu item
 */
export interface ContextMenuItem {
  label: string;
  action: () => void;
  disabled?: boolean;
}

/**
 * UI Store State
 */
interface UIStore {
  // Tabs
  tabs: TabInfo[];
  activeTabId: string | null;
  
  // Context Menu
  contextMenu: {
    visible: boolean;
    x: number;
    y: number;
    type: string;
    path: string;
    items: ContextMenuItem[];
  };
  
  // Actions - Tabs
  addTab: (tab: TabInfo) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  
  // Actions - Context Menu
  showContextMenu: (x: number, y: number, type: string, path: string, items: ContextMenuItem[]) => void;
  hideContextMenu: () => void;
}

/**
 * UI Store Implementation
 */
export const useUIStore = create<UIStore>((set, get) => ({
  // Initial state - Tabs
  tabs: [
    {
      id: 'welcome',
      title: 'Welcome',
      type: 'welcome'
    }
  ],
  activeTabId: 'welcome',
  
  // Initial state - Context Menu
  contextMenu: {
    visible: false,
    x: 0,
    y: 0,
    type: '',
    path: '',
    items: []
  },
  
  // Actions - Tabs
  addTab: (tab: TabInfo) => {
    set((state) => {
      // Check if tab with same id already exists
      const existingTabIndex = state.tabs.findIndex(t => t.id === tab.id);
      
      if (existingTabIndex !== -1) {
        // Tab exists, just activate it
        return { 
          ...state,
          activeTabId: tab.id 
        };
      }
      
      // Add new tab
      return { 
        ...state,
        tabs: [...state.tabs, tab],
        activeTabId: tab.id 
      };
    });
  },
  
  closeTab: (id: string) => {
    set((state) => {
      // Don't close the last tab
      if (state.tabs.length <= 1) {
        return state;
      }
      
      const newTabs = state.tabs.filter(tab => tab.id !== id);
      
      // If we're closing the active tab, activate another one
      let newActiveTabId = state.activeTabId;
      if (state.activeTabId === id) {
        newActiveTabId = newTabs[newTabs.length - 1].id;
      }
      
      return { 
        ...state,
        tabs: newTabs,
        activeTabId: newActiveTabId
      };
    });
  },
  
  setActiveTab: (id: string) => {
    set((state) => ({ 
      ...state, 
      activeTabId: id 
    }));
  },
  
  // Actions - Context Menu
  showContextMenu: (x, y, type, path, items) => {
    set((state) => ({ 
      ...state, 
      contextMenu: {
        visible: true,
        x,
        y,
        type,
        path,
        items
      }
    }));
  },
  
  hideContextMenu: () => {
    set((state) => ({ 
      ...state, 
      contextMenu: {
        ...state.contextMenu,
        visible: false
      }
    }));
  }
}));