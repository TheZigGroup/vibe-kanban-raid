import { useCallback } from 'react';
import { useSpeakerConfidenceStore } from '@/stores/useSpeakerConfidenceStore';
import { SpeakerConfidenceDialog } from '@/components/dialogs/tvw/SpeakerConfidenceDialog';
import type { TranscriptSegment } from '@/types/speaker-confidence';

/**
 * Hook for speaker confidence operations
 * Provides convenient methods for correcting speaker identification
 */
export function useSpeakerConfidence(videoId: string) {
  const transcript = useSpeakerConfidenceStore((s) => s.getTranscript(videoId));
  const correctSpeaker = useSpeakerConfidenceStore((s) => s.correctSpeaker);
  const updateSegment = useSpeakerConfidenceStore((s) => s.updateSegment);

  /**
   * Open speaker correction dialog for a segment
   */
  const openCorrectionDialog = useCallback(
    async (segmentId: string) => {
      if (!transcript) return;

      const segment = transcript.segments.find((s) => s.id === segmentId);
      if (!segment) return;

      const currentSpeaker = transcript.speakers.find(
        (s) => s.id === segment.speakerId
      );
      if (!currentSpeaker) return;

      const result = await SpeakerConfidenceDialog.show({
        segment,
        currentSpeaker,
        availableSpeakers: transcript.speakers,
        thresholds: transcript.confidenceThresholds,
      });

      if (result) {
        correctSpeaker(
          videoId,
          segmentId,
          result.newSpeakerId,
          result.markAsVerified
        );
      }
    },
    [transcript, videoId, correctSpeaker]
  );

  /**
   * Manually verify a segment without changing the speaker
   */
  const verifySegment = useCallback(
    (segmentId: string) => {
      updateSegment(videoId, segmentId, { isManuallyVerified: true });
    },
    [videoId, updateSegment]
  );

  /**
   * Get segments with low confidence that need review
   */
  const getLowConfidenceSegments = useCallback((): TranscriptSegment[] => {
    if (!transcript) return [];

    return transcript.segments.filter(
      (s) =>
        !s.isManuallyVerified &&
        s.confidence < transcript.confidenceThresholds.medium
    );
  }, [transcript]);

  /**
   * Get statistics about confidence levels
   */
  const getConfidenceStats = useCallback(() => {
    if (!transcript) {
      return {
        total: 0,
        high: 0,
        medium: 0,
        low: 0,
        verified: 0,
        needsReview: 0,
      };
    }

    const { segments, confidenceThresholds } = transcript;
    const stats = {
      total: segments.length,
      high: 0,
      medium: 0,
      low: 0,
      verified: 0,
      needsReview: 0,
    };

    segments.forEach((segment) => {
      if (segment.isManuallyVerified) {
        stats.verified++;
      } else if (segment.confidence < confidenceThresholds.medium) {
        stats.needsReview++;
      }

      if (segment.confidence >= confidenceThresholds.high) {
        stats.high++;
      } else if (segment.confidence >= confidenceThresholds.medium) {
        stats.medium++;
      } else {
        stats.low++;
      }
    });

    return stats;
  }, [transcript]);

  return {
    transcript,
    openCorrectionDialog,
    verifySegment,
    getLowConfidenceSegments,
    getConfidenceStats,
  };
}

/**
 * Hook for bulk speaker correction operations
 */
export function useBulkSpeakerCorrection(videoId: string) {
  const correctSpeaker = useSpeakerConfidenceStore((s) => s.correctSpeaker);
  const transcript = useSpeakerConfidenceStore((s) => s.getTranscript(videoId));

  /**
   * Correct multiple segments to the same speaker
   */
  const correctMultipleSegments = useCallback(
    (segmentIds: string[], newSpeakerId: string, markAsVerified: boolean) => {
      segmentIds.forEach((segmentId) => {
        correctSpeaker(videoId, segmentId, newSpeakerId, markAsVerified);
      });
    },
    [videoId, correctSpeaker]
  );

  /**
   * Auto-correct segments based on confidence threshold
   * This would be used with ML model improvements
   */
  const autoCorrectLowConfidence = useCallback(
    (threshold: number, newSpeakerId: string) => {
      if (!transcript) return;

      const lowConfidenceSegments = transcript.segments.filter(
        (s) => !s.isManuallyVerified && s.confidence < threshold
      );

      lowConfidenceSegments.forEach((segment) => {
        correctSpeaker(videoId, segment.id, newSpeakerId, false);
      });

      return lowConfidenceSegments.length;
    },
    [videoId, transcript, correctSpeaker]
  );

  return {
    correctMultipleSegments,
    autoCorrectLowConfidence,
  };
}
