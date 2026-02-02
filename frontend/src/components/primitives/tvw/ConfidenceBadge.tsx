import * as React from 'react';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import {
  type ConfidenceLevel,
  type SpeakerConfidenceThresholds,
  getConfidenceLevel,
  getConfidenceBadgeVariant,
} from '@/types/speaker-confidence';

export interface ConfidenceBadgeProps {
  /**
   * Confidence score (0-1 scale)
   */
  score: number;
  /**
   * Thresholds for determining confidence levels
   */
  thresholds: SpeakerConfidenceThresholds;
  /**
   * Whether to show the percentage
   */
  showPercentage?: boolean;
  /**
   * Whether the segment has been manually verified
   */
  isManuallyVerified?: boolean;
  /**
   * Additional className
   */
  className?: string;
}

/**
 * ConfidenceBadge displays speaker identification confidence
 * with color coding based on confidence levels.
 */
export function ConfidenceBadge({
  score,
  thresholds,
  showPercentage = true,
  isManuallyVerified = false,
  className,
}: ConfidenceBadgeProps) {
  const level = getConfidenceLevel(score, thresholds);
  const variant = getConfidenceBadgeVariant(level);
  const percentage = Math.round(score * 100);

  const content = React.useMemo(() => {
    if (isManuallyVerified) {
      return '✓ Verified';
    }
    if (showPercentage) {
      return `${percentage}%`;
    }
    return level.charAt(0).toUpperCase() + level.slice(1);
  }, [isManuallyVerified, showPercentage, percentage, level]);

  return (
    <Badge
      variant={isManuallyVerified ? 'default' : variant}
      className={cn(
        'text-[10px] px-1.5 py-0',
        isManuallyVerified && 'bg-green-600 hover:bg-green-700 border-green-500',
        className
      )}
    >
      {content}
    </Badge>
  );
}
