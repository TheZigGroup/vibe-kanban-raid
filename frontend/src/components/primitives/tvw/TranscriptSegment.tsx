import * as React from 'react';
import { cn } from '@/lib/utils';
import { SpeakerLabel } from './SpeakerLabel';
import { ConfidenceBadge } from './ConfidenceBadge';
import type {
  Speaker,
  TranscriptSegment as TranscriptSegmentType,
  SpeakerConfidenceThresholds,
} from '@/types/speaker-confidence';

export interface TranscriptSegmentProps {
  /**
   * Transcript segment data
   */
  segment: TranscriptSegmentType;
  /**
   * Speaker information
   */
  speaker: Speaker;
  /**
   * Confidence thresholds
   */
  thresholds: SpeakerConfidenceThresholds;
  /**
   * Whether this segment is currently active (playing)
   */
  isActive?: boolean;
  /**
   * Whether to show confidence badge
   */
  showConfidence?: boolean;
  /**
   * Click handler for editing speaker
   */
  onEditSpeaker?: (segmentId: string) => void;
  /**
   * Click handler for segment (e.g., to seek video)
   */
  onClick?: (segment: TranscriptSegmentType) => void;
  /**
   * Additional className
   */
  className?: string;
}

/**
 * Format seconds to MM:SS
 */
function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

/**
 * TranscriptSegment displays a single transcript line with speaker identification,
 * confidence indicator, and interactive controls.
 */
export function TranscriptSegment({
  segment,
  speaker,
  thresholds,
  isActive = false,
  showConfidence = true,
  onEditSpeaker,
  onClick,
  className,
}: TranscriptSegmentProps) {
  const handleSpeakerClick = React.useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onEditSpeaker?.(segment.id);
    },
    [segment.id, onEditSpeaker]
  );

  const handleSegmentClick = React.useCallback(() => {
    onClick?.(segment);
  }, [segment, onClick]);

  return (
    <div
      className={cn(
        'group px-4 py-3 border-l-2 transition-colors',
        isActive
          ? 'bg-brand/10 border-brand'
          : 'border-transparent hover:bg-bg-secondary',
        onClick && 'cursor-pointer',
        className
      )}
      onClick={handleSegmentClick}
    >
      {/* Header: Speaker + Time + Confidence */}
      <div className="flex items-center justify-between gap-2 mb-1.5">
        <div className="flex items-center gap-2">
          <SpeakerLabel
            speaker={speaker}
            size="sm"
            onClick={onEditSpeaker ? handleSpeakerClick : undefined}
          />
          {showConfidence && (
            <ConfidenceBadge
              score={segment.confidence}
              thresholds={thresholds}
              isManuallyVerified={segment.isManuallyVerified}
            />
          )}
        </div>
        <time className="text-xs text-text-low tabular-nums">
          {formatTime(segment.startTime)}
        </time>
      </div>

      {/* Transcript Text */}
      <p className="text-sm text-text-normal leading-relaxed">{segment.text}</p>
    </div>
  );
}
