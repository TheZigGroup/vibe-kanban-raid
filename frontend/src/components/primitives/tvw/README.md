# TVW (Transcript Video Widget) Components

Speaker confidence scoring UI components for displaying and correcting speaker identification in video transcripts.

## Components

### Primitive Components (Layer 1)

#### ConfidenceBadge
Displays speaker identification confidence with color coding.

```tsx
import { ConfidenceBadge } from '@/components/primitives/tvw';

<ConfidenceBadge
  score={0.85}
  thresholds={{ high: 0.8, medium: 0.5 }}
  showPercentage={true}
  isManuallyVerified={false}
/>
```

**Props:**
- `score`: Confidence score (0-1 scale)
- `thresholds`: Confidence level thresholds
- `showPercentage`: Whether to show percentage (default: true)
- `isManuallyVerified`: If segment was manually verified (default: false)

**Visual Behavior:**
- High confidence (≥80%): Green badge with percentage
- Medium confidence (≥50%): Yellow badge with percentage
- Low confidence (<50%): Red badge with percentage
- Manually verified: Green badge with "✓ Verified"

---

#### SpeakerLabel
Displays speaker name with optional color indicator.

```tsx
import { SpeakerLabel } from '@/components/primitives/tvw';

const speaker = {
  id: 'speaker-1',
  name: 'John Doe',
  color: '#3B82F6',
};

<SpeakerLabel
  speaker={speaker}
  showColorDot={true}
  size="md"
  onClick={() => console.log('Clicked')}
/>
```

**Props:**
- `speaker`: Speaker object with id, name, and color
- `showColorDot`: Show color indicator (default: true)
- `size`: 'sm' | 'md' | 'lg' (default: 'md')
- `onClick`: Optional click handler

---

#### TranscriptSegment
Complete transcript line with speaker, confidence, and text.

```tsx
import { TranscriptSegment } from '@/components/primitives/tvw';

const segment = {
  id: 'seg-1',
  speakerId: 'speaker-1',
  text: 'Hello, how are you today?',
  startTime: 45.2,
  endTime: 48.5,
  confidence: 0.92,
  isManuallyVerified: false,
};

<TranscriptSegment
  segment={segment}
  speaker={speaker}
  thresholds={{ high: 0.8, medium: 0.5 }}
  isActive={true}
  showConfidence={true}
  onEditSpeaker={(segmentId) => console.log('Edit', segmentId)}
  onClick={(segment) => console.log('Seek to', segment.startTime)}
/>
```

**Props:**
- `segment`: Transcript segment data
- `speaker`: Speaker information
- `thresholds`: Confidence thresholds
- `isActive`: If segment is currently playing (default: false)
- `showConfidence`: Show confidence badge (default: true)
- `onEditSpeaker`: Handler to edit speaker assignment
- `onClick`: Handler to seek video to this segment

---

### Dialogs

#### SpeakerConfidenceDialog
Modal for correcting speaker identification.

```tsx
import { SpeakerConfidenceDialog } from '@/components/dialogs/tvw';

const result = await SpeakerConfidenceDialog.show({
  segment,
  currentSpeaker,
  availableSpeakers: [speaker1, speaker2, speaker3],
  thresholds: { high: 0.8, medium: 0.5 },
});

if (result) {
  console.log('New speaker:', result.newSpeakerId);
  console.log('Verified:', result.markAsVerified);
}
```

---

## State Management

### Zustand Store

```tsx
import {
  useSpeakerConfidenceStore,
  useActiveTranscript,
  useFilteredSegments,
  useConfidenceThresholds,
} from '@/stores/useSpeakerConfidenceStore';

// Set active video
const setActiveVideo = useSpeakerConfidenceStore(s => s.setActiveVideo);
setActiveVideo('video-123');

// Get active transcript
const { activeVideoId, transcript } = useActiveTranscript();

// Filter segments by confidence
const setFilter = useSpeakerConfidenceStore(s => s.setFilterByConfidence);
setFilter('low'); // 'all' | 'low' | 'medium' | 'high' | 'unverified'
const segments = useFilteredSegments();

// Correct speaker
const correctSpeaker = useSpeakerConfidenceStore(s => s.correctSpeaker);
correctSpeaker('video-123', 'segment-1', 'new-speaker-id', true);
```

---

### Custom Hooks

```tsx
import { useSpeakerConfidence } from '@/hooks/useSpeakerConfidence';

function TranscriptPanel({ videoId }: { videoId: string }) {
  const {
    transcript,
    openCorrectionDialog,
    verifySegment,
    getLowConfidenceSegments,
    getConfidenceStats,
  } = useSpeakerConfidence(videoId);

  const stats = getConfidenceStats();
  console.log(`${stats.needsReview} segments need review`);

  return (
    <div>
      {transcript?.segments.map(segment => (
        <TranscriptSegment
          key={segment.id}
          segment={segment}
          speaker={transcript.speakers.find(s => s.id === segment.speakerId)!}
          thresholds={transcript.confidenceThresholds}
          onEditSpeaker={openCorrectionDialog}
        />
      ))}
    </div>
  );
}
```

---

## Complete Example

```tsx
import { useEffect } from 'react';
import { useSpeakerConfidenceStore } from '@/stores/useSpeakerConfidenceStore';
import { useSpeakerConfidence } from '@/hooks/useSpeakerConfidence';
import { TranscriptSegment } from '@/components/primitives/tvw';
import type { VideoTranscript } from '@/types/speaker-confidence';

export function VideoTranscriptView({ videoId }: { videoId: string }) {
  const setActiveVideo = useSpeakerConfidenceStore(s => s.setActiveVideo);
  const setTranscript = useSpeakerConfidenceStore(s => s.setTranscript);
  const transcript = useSpeakerConfidenceStore(s => s.getTranscript(videoId));

  const {
    openCorrectionDialog,
    getConfidenceStats,
  } = useSpeakerConfidence(videoId);

  // Initialize on mount
  useEffect(() => {
    setActiveVideo(videoId);

    // In real app, fetch from API
    const mockTranscript: VideoTranscript = {
      id: 'transcript-1',
      videoId,
      speakers: [
        { id: 'speaker-1', name: 'John Doe', color: '#3B82F6' },
        { id: 'speaker-2', name: 'Jane Smith', color: '#EF4444' },
      ],
      segments: [
        {
          id: 'seg-1',
          speakerId: 'speaker-1',
          text: 'Welcome to our meeting today.',
          startTime: 0,
          endTime: 2.5,
          confidence: 0.95,
          isManuallyVerified: false,
        },
        {
          id: 'seg-2',
          speakerId: 'speaker-2',
          text: 'Thanks for having me.',
          startTime: 2.5,
          endTime: 4.0,
          confidence: 0.65,
          isManuallyVerified: false,
        },
      ],
      confidenceThresholds: { high: 0.8, medium: 0.5 },
    };

    setTranscript(videoId, mockTranscript);
  }, [videoId, setActiveVideo, setTranscript]);

  if (!transcript) return <div>Loading...</div>;

  const stats = getConfidenceStats();

  return (
    <div className="flex flex-col h-full">
      {/* Stats Header */}
      <div className="p-4 border-b">
        <div className="flex gap-4 text-sm">
          <span>Total: {stats.total}</span>
          <span className="text-green-600">High: {stats.high}</span>
          <span className="text-yellow-600">Medium: {stats.medium}</span>
          <span className="text-red-600">Low: {stats.low}</span>
          <span className="text-blue-600">Verified: {stats.verified}</span>
        </div>
      </div>

      {/* Transcript List */}
      <div className="flex-1 overflow-y-auto">
        {transcript.segments.map((segment) => {
          const speaker = transcript.speakers.find(
            s => s.id === segment.speakerId
          );
          if (!speaker) return null;

          return (
            <TranscriptSegment
              key={segment.id}
              segment={segment}
              speaker={speaker}
              thresholds={transcript.confidenceThresholds}
              onEditSpeaker={openCorrectionDialog}
            />
          );
        })}
      </div>
    </div>
  );
}
```

---

## Type Definitions

See `@/types/speaker-confidence.ts` for complete type definitions:

- `Speaker` - Speaker identification
- `TranscriptSegment` - Single transcript line
- `VideoTranscript` - Complete transcript data
- `ConfidenceLevel` - 'high' | 'medium' | 'low'
- `SpeakerConfidenceThresholds` - Confidence boundaries
- `SpeakerCorrectionData` - Correction tracking

---

## Architecture

This implementation follows the codebase's established patterns:

1. **Layer 1 (Primitives)**: Pure UI components with minimal logic
2. **Layer 2 (Dialogs)**: NiceModal-based modals for user interactions
3. **State Management**: Zustand store with persistence
4. **Custom Hooks**: Business logic abstraction
5. **Styling**: Tailwind CSS with shadcn/ui components

---

## Future Enhancements

- Bulk correction operations UI
- Confidence threshold adjustment UI
- Speaker profile management
- Export correction data for ML training
- Real-time confidence updates during playback
- Keyboard shortcuts for quick corrections
