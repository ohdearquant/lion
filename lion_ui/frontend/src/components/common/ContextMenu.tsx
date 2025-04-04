import React, { useEffect, useRef } from 'react';
import { ContextMenuItem } from '../../stores/uiStore';

interface ContextMenuProps {
  visible: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

/**
 * Context Menu component
 * Purpose: Reusable context menu for right-click operations
 * Props: 
 *   - visible: boolean
 *   - x: number
 *   - y: number
 *   - items: { label: string, action: () => void, disabled?: boolean }[]
 *   - onClose: () => void
 * State: None
 * API Calls: None (uses callbacks)
 */
export const ContextMenu: React.FC<ContextMenuProps> = ({ 
  visible, 
  x, 
  y, 
  items, 
  onClose 
}) => {
  const menuRef = useRef<HTMLDivElement>(null);
  
  // Close the menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose();
      }
    };
    
    if (visible) {
      document.addEventListener('mousedown', handleClickOutside);
    }
    
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [visible, onClose]);
  
  // Close the menu when pressing Escape
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };
    
    if (visible) {
      document.addEventListener('keydown', handleKeyDown);
    }
    
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [visible, onClose]);
  
  if (!visible) {
    return null;
  }
  
  // Handle menu item click
  const handleItemClick = (item: ContextMenuItem) => {
    if (!item.disabled) {
      item.action();
      onClose();
    }
  };
  
  return (
    <div 
      ref={menuRef}
      className="absolute bg-white dark:bg-gray-800 shadow-lg rounded border border-gray-200 dark:border-gray-700 z-50 py-1"
      style={{ 
        left: x, 
        top: y,
        minWidth: '160px',
        maxWidth: '240px'
      }}
    >
      {items.map((item, index) => (
        <div 
          key={index}
          className={`px-4 py-2 text-sm cursor-pointer ${
            item.disabled 
              ? 'text-gray-400 dark:text-gray-500' 
              : 'text-gray-800 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700'
          }`}
          onClick={() => handleItemClick(item)}
        >
          {item.label}
        </div>
      ))}
    </div>
  );
};