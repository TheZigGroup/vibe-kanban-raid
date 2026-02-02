import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type {
  Speaker,
  TranscriptSegment,
  SpeakerConfidenceThresholds,
  SpeakerCorrectionData,
  VideoTranscript,
} from '@/types/speaker-confidence';

/**
 * Default confidence thresholds
 * - high: >= 0.8 (80%)
 * - medium: >= 0.5 (50%)
 * - low: < 0.5 (50%)
 */
const DEFAULT_THRESHOLDS: SpeakerConfidenceThresholds = {
  high: 0.8,
  medium: 0.5,
};

type State = {
  // Video transcripts keyed by videoId
  transcripts: Record<string, VideoTranscript>;

  // Global confidence thresholds (can be overridden per video)
  globalThresholds: SpeakerConfidenceThresholds;

  // Track corrections for learning
  corrections: Record<string, SpeakerCorrectionData[]>; // keyed by videoId

  // UI state
  activeVideoId: string | null;
  filterByConfidence: 'all' | 'low' | 'medium' | 'high' | 'unverified';

  // Actions
  setActiveVideo: (videoId: string | null) => void;
  setFilterByConfidence: (filter: State['filterByConfidence']) => void;

  // Transcript management
  setTranscript: (videoId: string, transcript: VideoTranscript) => void;
  updateSegment: (
    videoId: string,
    segmentId: string,
    updates: Partial<TranscriptSegment>
  ) => void;

  // Speaker correction
  correctSpeaker: (
    videoId: string,
    segmentId: string,
    newSpeakerId: string,
    markAsVerified: boolean
  ) => void;

  // Threshold management
  setGlobalThresholds: (thresholds: SpeakerConfidenceThresholds) => void;
  setVideoThresholds: (
    videoId: string,
    thresholds: SpeakerConfidenceThresholds
  ) => void;

  // Computed getters
  getTranscript: (videoId: string) => VideoTranscript | undefined;
  getSegment: (videoId: string, segmentId: string) => TranscriptSegment | undefined;
  getSpeaker: (videoId: string, speakerId: string) => Speaker | undefined;
  getFilteredSegments: (videoId: string) => TranscriptSegment[];
  getCorrections: (videoId: string) => SpeakerCorrectionData[];
};

export const useSpeakerConfidenceStore = create<State>()(
  persist(
    (set, get) => ({
      // Initial state
      transcripts: {},
      globalThresholds: DEFAULT_THRESHOLDS,
      corrections: {},
      activeVideoId: null,
      filterByConfidence: 'all',

      // UI actions
      setActiveVideo: (videoId) => set({ activeVideoId: videoId }),
      setFilterByConfidence: (filter) => set({ filterByConfidence: filter }),

      // Transcript management
      setTranscript: (videoId, transcript) =>
        set((state) => ({
          transcripts: {
            ...state.transcripts,
            [videoId]: transcript,
          },
        })),

      updateSegment: (videoId, segmentId, updates) =>
        set((state) => {
          const transcript = state.transcripts[videoId];
          if (!transcript) return state;

          return {
            transcripts: {
              ...state.transcripts,
              [videoId]: {
                ...transcript,
                segments: transcript.segments.map((segment) =>
                  segment.id === segmentId
                    ? { ...segment, ...updates }
                    : segment
                ),
              },
            },
          };
        }),

      // Speaker correction
      correctSpeaker: (videoId, segmentId, newSpeakerId, markAsVerified) =>
        set((state) => {
          const transcript = state.transcripts[videoId];
          if (!transcript) return state;

          const segment = transcript.segments.find((s) => s.id === segmentId);
          if (!segment) return state;

          const correction: SpeakerCorrectionData = {
            segmentId,
            oldSpeakerId: segment.speakerId,
            newSpeakerId,
            correctedAt: new Date(),
          };

          return {
            transcripts: {
              ...state.transcripts,
              [videoId]: {
                ...transcript,
                segments: transcript.segments.map((s) =>
                  s.id === segmentId
                    ? {
                        ...s,
                        speakerId: newSpeakerId,
                        isManuallyVerified: markAsVerified,
                      }
                    : s
                ),
              },
            },
            corrections: {
              ...state.corrections,
              [videoId]: [...(state.corrections[videoId] || []), correction],
            },
          };
        }),

      // Threshold management
      setGlobalThresholds: (thresholds) =>
        set({ globalThresholds: thresholds }),

      setVideoThresholds: (videoId, thresholds) =>
        set((state) => {
          const transcript = state.transcripts[videoId];
          if (!transcript) return state;

          return {
            transcripts: {
              ...state.transcripts,
              [videoId]: {
                ...transcript,
                confidenceThresholds: thresholds,
              },
            },
          };
        }),

      // Computed getters
      getTranscript: (videoId) => get().transcripts[videoId],

      getSegment: (videoId, segmentId) => {
        const transcript = get().transcripts[videoId];
        return transcript?.segments.find((s) => s.id === segmentId);
      },

      getSpeaker: (videoId, speakerId) => {
        const transcript = get().transcripts[videoId];
        return transcript?.speakers.find((s) => s.id === speakerId);
      },

      getFilteredSegments: (videoId) => {
        const state = get();
        const transcript = state.transcripts[videoId];
        if (!transcript) return [];

        const { segments, confidenceThresholds } = transcript;
        const filter = state.filterByConfidence;

        if (filter === 'all') return segments;
        if (filter === 'unverified') {
          return segments.filter((s) => !s.isManuallyVerified);
        }

        // Filter by confidence level
        return segments.filter((s) => {
          if (s.confidence >= confidenceThresholds.high) {
            return filter === 'high';
          }
          if (s.confidence >= confidenceThresholds.medium) {
            return filter === 'medium';
          }
          return filter === 'low';
        });
      },

      getCorrections: (videoId) => get().corrections[videoId] || [],
    }),
    {
      name: 'speaker-confidence',
      // Only persist thresholds and corrections, not full transcripts
      partialize: (state) => ({
        globalThresholds: state.globalThresholds,
        corrections: state.corrections,
      }),
    }
  )
);

/**
 * Hook to get active video's transcript
 */
export function useActiveTranscript() {
  const activeVideoId = useSpeakerConfidenceStore((s) => s.activeVideoId);
  const transcript = useSpeakerConfidenceStore((s) =>
    activeVideoId ? s.transcripts[activeVideoId] : undefined
  );
  return { activeVideoId, transcript };
}

/**
 * Hook to get filtered segments for active video
 */
export function useFilteredSegments() {
  const activeVideoId = useSpeakerConfidenceStore((s) => s.activeVideoId);
  const getFilteredSegments = useSpeakerConfidenceStore(
    (s) => s.getFilteredSegments
  );
  return activeVideoId ? getFilteredSegments(activeVideoId) : [];
}

/**
 * Hook to get confidence thresholds for a video
 */
export function useConfidenceThresholds(videoId: string | null) {
  const globalThresholds = useSpeakerConfidenceStore((s) => s.globalThresholds);
  const videoThresholds = useSpeakerConfidenceStore((s) =>
    videoId ? s.transcripts[videoId]?.confidenceThresholds : undefined
  );
  return videoThresholds || globalThresholds;
}
