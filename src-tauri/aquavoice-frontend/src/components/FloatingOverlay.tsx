import React, { useState, useRef, useEffect } from 'react';
import { Mic } from 'lucide-react';
import { motion } from 'framer-motion';

// Import Tauri functions for window dragging
declare global {
  interface Window {
    __TAURI__?: {
      core: {
        invoke: (cmd: string, args?: any) => Promise<any>;
      };
      window: {
        getCurrent: () => {
          startDragging: () => Promise<void>;
        };
      };
    };
  }
}

interface FloatingOverlayProps {
  onStartRecording: () => void;
  onStopRecording: () => void;
  isRecording: boolean;
  transcript: string;
  status: string;
  onCopyText: () => void;
}

export const FloatingOverlay: React.FC<FloatingOverlayProps> = ({
  onStartRecording,
  onStopRecording,
  isRecording,
  transcript,
  status,
  onCopyText,
}) => {
  const [clickCount, setClickCount] = useState(0);
  const clickTimer = useRef<NodeJS.Timeout | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [currentStyle, setCurrentStyle] = useState(3); // Start with Style 3 (Enhanced Neon - Your Favorite!)
  const buttonRef = useRef<HTMLButtonElement>(null);
  const [barHeights, setBarHeights] = useState([0.3, 0.6, 0.8, 0.4]);
  const animationRef = useRef<number | null>(null);
  
  // Tauri invoke function
  const isTauri = window.__TAURI__ !== undefined;
  const tauriInvoke = isTauri ? window.__TAURI__!.core.invoke : null;

  const handleClick = () => {
    if (isDragging) {
      console.log('🚫 Click ignored - dragging in progress');
      return; // Don't trigger clicks during drag
    }
    
    setClickCount(prev => prev + 1);
    
    if (clickTimer.current) {
      clearTimeout(clickTimer.current);
    }
    
    clickTimer.current = setTimeout(() => {
      if (clickCount === 1 && isRecording) {
        // Single click while recording - stop
        console.log('🛑 Single click - stopping recording');
        onStopRecording();
      } else if (clickCount >= 2 && !isRecording) {
        // Double click when not recording - start
        console.log('🎤 Double click - starting recording');
        onStartRecording();
      } else if (clickCount === 1 && !isRecording && transcript) {
        // Single click with transcript - copy to clipboard
        console.log('📋 Single click - copying to clipboard');
        onCopyText();
      }
      setClickCount(0);
    }, 400);
  };

  const handleMouseDown = async (e: React.MouseEvent) => {
    console.log(`🖱️ Mouse down - button: ${e.button}, ctrl: ${e.ctrlKey}, shift: ${e.shiftKey}`);
    
    // Right click OR Ctrl+left click to drag - Check for primary button (left click) for faster response
    if (e.button === 2 || (e.button === 0 && e.ctrlKey)) {
      e.preventDefault();
      e.stopPropagation();
      setIsDragging(true);
      
      console.log('🖱️ Drag gesture detected, starting drag...');
      
      try {
        // Use the correct Tauri v2 API from official docs
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        console.log('📦 Tauri getCurrentWindow API loaded');
        
        const appWindow = getCurrentWindow();
        console.log('🪟 Got current window handle');
        
        // Call startDragging() as per official Tauri v2 docs
        console.log('🚀 Calling startDragging()...');
        await appWindow.startDragging();
        console.log('✅ Window dragging initiated successfully');
      } catch (error) {
        console.error('❌ Window dragging failed:', error);
        console.error('❌ Error details:', (error as Error)?.message);
        
        // Check if it's a permission error
        if ((error as Error)?.message?.includes('permission')) {
          console.error('🚫 Permission denied - check capabilities/default.json');
        }
      }
      
      setTimeout(() => setIsDragging(false), 200);
    }
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault(); // Disable context menu on right click
    e.stopPropagation();
  };

  const getButtonState = () => {
    // FIXED LOGIC - EXPLICIT CHECKS TO AVOID STUCK STATES
    if (isRecording) {
      console.log('🔴 RECORDING STATE');
      return 'recording';
    }
    
    if ((status.includes('Processing') || status.includes('Transcribing')) && !status.includes('Ready')) {
      console.log('🟡 PROCESSING STATE');
      return 'processing';
    }
    
    console.log('🔵 READY STATE - CLEAN BLUE');
    return 'ready';
  };

  // REMOVED CONFLICTING SPACEBAR LISTENER - Let App.tsx handle it naturally

  // REAL-TIME AUDIO VISUALIZATION - USING TAURI BACKEND AUDIO LEVELS
  useEffect(() => {
    if (isRecording) {
      console.log('🎤 STARTING REAL-TIME AUDIO VISUALIZATION FROM TAURI');
      startRealAudioVisualization();
    } else {
      console.log('🛑 STOPPING AUDIO VISUALIZATION');
      stopRealAudioVisualization();
      setBarHeights([0.2, 0.2, 0.2, 0.2]); // Reset to silent state
    }
    
    // Cleanup on unmount
    return () => {
      stopRealAudioVisualization();
    };
  }, [isRecording]);

  const startRealAudioVisualization = () => {
    // Start polling Tauri backend for REAL audio levels
    const updateAudioLevels = async () => {
      try {
        if (isTauri && tauriInvoke) {
          const levels = await tauriInvoke('get_audio_levels') as number[];
          console.log('🎵 Real audio levels:', levels);
          
          // Convert to visualization scale (multiply by 8 for better visual effect)
          const visualHeights = levels.map(level => Math.max(level * 1.2, 0.2)); // Min 0.2, scale by 1.2
          setBarHeights(visualHeights);
        }
      } catch (error) {
        console.error('❌ Failed to get audio levels:', error);
        // Fallback to static levels
        setBarHeights([0.2, 0.2, 0.2, 0.2]);
      }
    };
    
    // Poll every 50ms for smooth real-time visualization
    updateAudioLevels(); // Initial call
    animationRef.current = setInterval(updateAudioLevels, 50) as any;
  };

  const stopRealAudioVisualization = () => {
    if (animationRef.current) {
      clearInterval(animationRef.current);
      animationRef.current = null;
    }
  };

  // Style switcher - Ctrl+1-5 to switch styles
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key >= '1' && e.key <= '5') {
        e.preventDefault();
        const styleNum = parseInt(e.key);
        setCurrentStyle(styleNum);
        console.log(`🎨 Switched to Style ${styleNum}`);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <div 
      className="w-full h-full flex items-center justify-center"
      onMouseDown={handleMouseDown}
      onContextMenu={handleContextMenu}
      data-tauri-drag-region
    >
      {/* Dynamic Island Button - Exact size match to Tauri window */}
      <motion.button
        ref={buttonRef}
        onClick={handleClick}
        className={`island-button style${currentStyle} ${getButtonState()}`}
        style={{
          width: '80px',  // Increased by 11% (72 -> 80)
          height: '24px', // Increased by 18% (20 -> 24) 
          borderRadius: '12px', // Scaled proportionally
        }}
        initial={{ scale: 0.9, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        whileHover={{ scale: currentStyle === 3 ? 1.02 : 1.05, y: currentStyle === 3 ? -1 : -2 }}
        whileTap={{ scale: 0.97 }}
        transition={{ type: 'spring', stiffness: 300, damping: 30 }}
        title={
          isRecording 
            ? "Recording... • Click or Spacebar to stop • Right-click to drag"
            : transcript 
              ? "Single click to inject text • Double-click or Alt+Alt to record • Right-click to drag"
              : `Style ${currentStyle}/5 • Double-click or Alt+Alt to record • Ctrl+1-5 to switch styles • Right-click to drag`
        }
      >
        <div className="flex items-center justify-center gap-1 text-white font-semibold text-[7px]">
          {isRecording ? (
            <div className="flex items-end justify-center gap-0.5 h-2">
              <div 
                className="w-0.5 bg-red-500 rounded-sm transition-all duration-75" 
                style={{ height: `${barHeights[0] * 8}px` }}
              ></div>
              <div 
                className="w-0.5 bg-red-500 rounded-sm transition-all duration-75" 
                style={{ height: `${barHeights[1] * 8}px` }}
              ></div>
              <div 
                className="w-0.5 bg-red-500 rounded-sm transition-all duration-75" 
                style={{ height: `${barHeights[2] * 8}px` }}
              ></div>
              <div 
                className="w-0.5 bg-red-500 rounded-sm transition-all duration-75" 
                style={{ height: `${barHeights[3] * 8}px` }}
              ></div>
            </div>
          ) : (status.includes('Processing') || status.includes('Transcribing')) && !status.includes('Ready') ? (
            <>
              <div className="w-1 h-1 bg-orange-300 rounded-full animate-pulse" />
              <span>PROC</span>
            </>
          ) : transcript ? (
            <>
              <div className="w-1 h-1 bg-green-400 rounded-full" />
              <span>READY</span>
            </>
          ) : (
            <>
              <Mic className="w-2 h-2" />
              <span>TALK</span>
            </>
          )}
        </div>
      </motion.button>
      
      {/* Auto-inject removed - will be manual only */}
    </div>
  );
};