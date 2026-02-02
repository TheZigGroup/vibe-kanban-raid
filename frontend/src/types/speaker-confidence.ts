/**
 * Speaker confidence types for TVW (Transcript Video Widget)
 */

export type ConfidenceLevel = 'high' | 'medium' | 'low';

export interface Speaker {
  id: string;
  name: string;
  color: string; // Hex color for visual identification
}

export interface TranscriptSegment {
  id: string;
  speakerId: string;
  text: string;
  startTime: number; // in seconds
  endTime: number; // in seconds
  confidence: number; // 0-1 scale
  isManuallyVerified: boolean;
}

export interface SpeakerConfidenceThresholds {
  high: number; // e.g., 0.8
  medium: number; // e.g., 0.5
}

export interface SpeakerCorrectionData {
  segmentId: string;
  oldSpeakerId: string;
  newSpeakerId: string;
  correctedAt: Date;
}

export interface VideoTranscript {
  id: string;
  videoId: string;
  speakers: Speaker[];
  segments: TranscriptSegment[];
  confidenceThresholds: SpeakerConfidenceThresholds;
}

/**
 * Helper to determine confidence level based on score and thresholds
 */
export function getConfidenceLevel(
  score: number,
  thresholds: SpeakerConfidenceThresholds
): ConfidenceLevel {
  if (score >= thresholds.high) return 'high';
  if (score >= thresholds.medium) return 'medium';
  return 'low';
}

/**
 * Helper to get confidence badge color
 */
export function getConfidenceBadgeVariant(level: ConfidenceLevel): 'default' | 'secondary' | 'destructive' {
  switch (level) {
    case 'high':
      return 'default';
    case 'medium':
      return 'secondary';
    case 'low':
      return 'destructive';
  }
}
