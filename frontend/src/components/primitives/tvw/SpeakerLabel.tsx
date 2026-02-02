import * as React from 'react';
import { cn } from '@/lib/utils';
import type { Speaker } from '@/types/speaker-confidence';

export interface SpeakerLabelProps {
  /**
   * Speaker information
   */
  speaker: Speaker;
  /**
   * Whether to show the color dot
   */
  showColorDot?: boolean;
  /**
   * Size variant
   */
  size?: 'sm' | 'md' | 'lg';
  /**
   * Additional className
   */
  className?: string;
  /**
   * Click handler
   */
  onClick?: () => void;
}

const sizeClasses = {
  sm: 'text-xs',
  md: 'text-sm',
  lg: 'text-base',
};

const dotSizeClasses = {
  sm: 'w-2 h-2',
  md: 'w-2.5 h-2.5',
  lg: 'w-3 h-3',
};

/**
 * SpeakerLabel displays speaker identification with optional color indicator.
 * Used throughout transcript UI to identify who is speaking.
 */
export function SpeakerLabel({
  speaker,
  showColorDot = true,
  size = 'md',
  className,
  onClick,
}: SpeakerLabelProps) {
  const Component = onClick ? 'button' : 'div';

  return (
    <Component
      onClick={onClick}
      className={cn(
        'inline-flex items-center gap-1.5 font-medium',
        sizeClasses[size],
        onClick && 'cursor-pointer hover:opacity-70 transition-opacity',
        className
      )}
    >
      {showColorDot && (
        <span
          className={cn('rounded-full flex-shrink-0', dotSizeClasses[size])}
          style={{ backgroundColor: speaker.color }}
          aria-hidden="true"
        />
      )}
      <span>{speaker.name}</span>
    </Component>
  );
}
