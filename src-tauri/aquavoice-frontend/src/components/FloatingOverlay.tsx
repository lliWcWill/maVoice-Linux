import React, { useState, useRef, useEffect } from 'react';
import { Mic, MicOff, Settings, Copy, Send } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

interface FloatingOverlayProps {
  onStartRecording: () => void;
  onStopRecording: () => void;
  isRecording: boolean;
  transcript: string;
  status: string;
  onInjectText: () => void;
  onCopyText: () => void;
  onOpenSettings: () => void;
}

export const FloatingOverlay: React.FC<FloatingOverlayProps> = ({
  onStartRecording,
  onStopRecording,
  isRecording,
  transcript,
  status,
  onInjectText,
  onCopyText,
  onOpenSettings,
}) => {
  const [isDragging, setIsDragging] = useState(false);
  const [position, setPosition] = useState({ x: 200, y: 100 }); // Start at absolute position
  const [isExpanded, setIsExpanded] = useState(false);
  const dragRef = useRef<HTMLDivElement>(null);
  const dragStart = useRef({ x: 0, y: 0 });

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.target === dragRef.current || (e.target as HTMLElement).closest('.drag-handle')) {
      setIsDragging(true);
      dragStart.current = {
        x: e.clientX - position.x,
        y: e.clientY - position.y,
      };
    }
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (isDragging) {
      const newX = e.clientX - dragStart.current.x;
      const newY = e.clientY - dragStart.current.y;
      
      // Use absolute positioning for true desktop floating
      setPosition({
        x: Math.max(0, Math.min(window.innerWidth - 320, newX)),
        y: Math.max(0, Math.min(window.innerHeight - 100, newY)),
      });
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  useEffect(() => {
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging]);

  const handleRecordClick = () => {
    if (isRecording) {
      onStopRecording();
    } else {
      onStartRecording();
      setIsExpanded(true);
    }
  };

  const getStatusColor = () => {
    if (isRecording) return 'border-red-400 glow-red';
    if (status.includes('success') || status.includes('Transcribed')) return 'border-green-400 glow-green';
    if (status.includes('error') || status.includes('Error')) return 'border-red-400 glow-red';
    return 'border-blue-400 glow-blue';
  };

  return (
    <>
      {/* Main floating bar */}
      <motion.div
        ref={dragRef}
        className={`fixed z-50 ${isDragging ? 'cursor-grabbing' : 'cursor-grab'}`}
        style={{
          left: `${position.x}px`,
          top: `${position.y}px`,
        }}
        onMouseDown={handleMouseDown}
        initial={{ scale: 0.8, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ type: 'spring', stiffness: 300, damping: 30 }}
      >
        {/* Compact bar */}
        <motion.div
          className={`glass rounded-full px-6 py-3 floating-shadow transition-all duration-300 ${getStatusColor()}`}
          animate={{
            width: isExpanded ? 'auto' : '64px',
            paddingLeft: isExpanded ? '24px' : '12px',
            paddingRight: isExpanded ? '24px' : '12px',
          }}
        >
          <div className="flex items-center gap-3 drag-handle">
            {/* Record button */}
            <motion.button
              onClick={handleRecordClick}
              className={`w-8 h-8 rounded-full flex items-center justify-center transition-all duration-200 ${
                isRecording 
                  ? 'bg-red-500 animate-pulse-glow' 
                  : 'bg-blue-500 hover:bg-blue-600'
              }`}
              whileHover={{ scale: 1.1 }}
              whileTap={{ scale: 0.95 }}
            >
              {isRecording ? (
                <MicOff className="w-4 h-4 text-white" />
              ) : (
                <Mic className="w-4 h-4 text-white" />
              )}
            </motion.button>

            {/* Expandable content */}
            <AnimatePresence>
              {isExpanded && (
                <motion.div
                  className="flex items-center gap-2"
                  initial={{ opacity: 0, width: 0 }}
                  animate={{ opacity: 1, width: 'auto' }}
                  exit={{ opacity: 0, width: 0 }}
                  transition={{ duration: 0.3 }}
                >
                  {/* Status indicator */}
                  <span className="text-white text-sm font-medium min-w-0">
                    {status}
                  </span>

                  {/* Action buttons */}
                  {transcript && (
                    <>
                      <motion.button
                        onClick={onCopyText}
                        className="w-7 h-7 rounded-md bg-white/20 hover:bg-white/30 flex items-center justify-center transition-colors"
                        whileHover={{ scale: 1.05 }}
                        whileTap={{ scale: 0.95 }}
                        title="Copy transcript"
                      >
                        <Copy className="w-3.5 h-3.5 text-white" />
                      </motion.button>

                      <motion.button
                        onClick={onInjectText}
                        className="w-7 h-7 rounded-md bg-white/20 hover:bg-white/30 flex items-center justify-center transition-colors"
                        whileHover={{ scale: 1.05 }}
                        whileTap={{ scale: 0.95 }}
                        title="Inject text"
                      >
                        <Send className="w-3.5 h-3.5 text-white" />
                      </motion.button>
                    </>
                  )}

                  {/* Settings button */}
                  <motion.button
                    onClick={onOpenSettings}
                    className="w-7 h-7 rounded-md bg-white/20 hover:bg-white/30 flex items-center justify-center transition-colors"
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    title="Settings"
                  >
                    <Settings className="w-3.5 h-3.5 text-white" />
                  </motion.button>

                  {/* Collapse button */}
                  <motion.button
                    onClick={() => setIsExpanded(false)}
                    className="w-5 h-5 rounded-full bg-white/20 hover:bg-white/30 flex items-center justify-center transition-colors ml-1"
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    title="Collapse"
                  >
                    <span className="text-white text-xs font-bold">×</span>
                  </motion.button>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </motion.div>

        {/* Transcript popup */}
        <AnimatePresence>
          {transcript && isExpanded && (
            <motion.div
              className="absolute bottom-full mb-2 left-1/2 transform -translate-x-1/2 glass rounded-lg p-4 max-w-xs floating-shadow"
              initial={{ opacity: 0, y: 10, scale: 0.9 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 10, scale: 0.9 }}
              transition={{ type: 'spring', stiffness: 300, damping: 30 }}
            >
              <p className="text-white text-sm leading-relaxed">{transcript}</p>
              <div className="absolute top-full left-1/2 transform -translate-x-1/2 w-0 h-0 border-l-4 border-r-4 border-t-4 border-transparent border-t-white/20"></div>
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>

      {/* Recording pulse indicator */}
      <AnimatePresence>
        {isRecording && (
          <motion.div
            className="fixed inset-0 pointer-events-none z-40"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
          >
            <div className="absolute top-4 left-1/2 transform -translate-x-1/2 glass rounded-full px-4 py-2">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-red-400 rounded-full animate-pulse"></div>
                <span className="text-white text-sm font-medium">Recording...</span>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
};