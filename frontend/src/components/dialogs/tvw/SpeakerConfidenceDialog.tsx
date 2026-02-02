import * as React from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal, type SaveResult } from '@/lib/modals';
import { SpeakerLabel } from '@/components/primitives/tvw/SpeakerLabel';
import { ConfidenceBadge } from '@/components/primitives/tvw/ConfidenceBadge';
import type {
  Speaker,
  TranscriptSegment,
  SpeakerConfidenceThresholds,
} from '@/types/speaker-confidence';

export interface SpeakerConfidenceDialogProps {
  /**
   * The transcript segment being edited
   */
  segment: TranscriptSegment;
  /**
   * Current speaker assignment
   */
  currentSpeaker: Speaker;
  /**
   * All available speakers
   */
  availableSpeakers: Speaker[];
  /**
   * Confidence thresholds
   */
  thresholds: SpeakerConfidenceThresholds;
}

export interface SpeakerCorrectionResult {
  newSpeakerId: string;
  markAsVerified: boolean;
}

const SpeakerConfidenceDialogImpl = NiceModal.create<
  SpeakerConfidenceDialogProps
>((props) => {
  const modal = useModal();
  const { segment, currentSpeaker, availableSpeakers, thresholds } = props;

  const [selectedSpeakerId, setSelectedSpeakerId] = React.useState(
    currentSpeaker.id
  );
  const [markAsVerified, setMarkAsVerified] = React.useState(false);

  const handleSave = () => {
    const result: SpeakerCorrectionResult = {
      newSpeakerId: selectedSpeakerId,
      markAsVerified,
    };
    modal.resolve(result);
  };

  const handleCancel = () => {
    modal.resolve(null);
  };

  const selectedSpeaker = availableSpeakers.find(
    (s) => s.id === selectedSpeakerId
  );

  const hasChanged = selectedSpeakerId !== currentSpeaker.id;

  return (
    <Dialog open={modal.visible} onOpenChange={handleCancel}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Edit Speaker Identification</DialogTitle>
          <DialogDescription>
            Correct the speaker for this transcript segment and improve future
            identification accuracy.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Current Confidence */}
          <div className="space-y-2">
            <Label className="text-sm font-medium text-text-normal">
              Current Confidence
            </Label>
            <div className="flex items-center gap-2">
              <ConfidenceBadge
                score={segment.confidence}
                thresholds={thresholds}
                showPercentage={true}
                isManuallyVerified={segment.isManuallyVerified}
              />
              <span className="text-sm text-text-low">
                {segment.isManuallyVerified
                  ? 'Manually verified'
                  : `AI confidence: ${Math.round(segment.confidence * 100)}%`}
              </span>
            </div>
          </div>

          {/* Transcript Text */}
          <div className="space-y-2">
            <Label className="text-sm font-medium text-text-normal">
              Transcript
            </Label>
            <div className="p-3 rounded-md bg-bg-secondary border border-border">
              <p className="text-sm text-text-normal leading-relaxed">
                "{segment.text}"
              </p>
            </div>
          </div>

          {/* Speaker Selection */}
          <div className="space-y-2">
            <Label htmlFor="speaker-select" className="text-sm font-medium">
              Correct Speaker
            </Label>
            <Select value={selectedSpeakerId} onValueChange={setSelectedSpeakerId}>
              <SelectTrigger id="speaker-select" className="w-full">
                <SelectValue>
                  {selectedSpeaker && (
                    <SpeakerLabel speaker={selectedSpeaker} size="sm" />
                  )}
                </SelectValue>
              </SelectTrigger>
              <SelectContent>
                {availableSpeakers.map((speaker) => (
                  <SelectItem key={speaker.id} value={speaker.id}>
                    <SpeakerLabel speaker={speaker} size="sm" />
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Mark as Verified Option */}
          {hasChanged && (
            <div className="flex items-center gap-2 p-3 rounded-md bg-brand/5 border border-brand/20">
              <input
                type="checkbox"
                id="mark-verified"
                checked={markAsVerified}
                onChange={(e) => setMarkAsVerified(e.target.checked)}
                className="h-4 w-4 rounded border-border text-brand focus:ring-brand"
              />
              <Label
                htmlFor="mark-verified"
                className="text-sm font-normal cursor-pointer"
              >
                Mark this correction as verified to improve AI accuracy
              </Label>
            </div>
          )}
        </div>

        <DialogFooter className="gap-2">
          <Button variant="outline" onClick={handleCancel}>
            Cancel
          </Button>
          <Button
            onClick={handleSave}
            disabled={!hasChanged && !segment.isManuallyVerified}
          >
            {hasChanged ? 'Save Correction' : 'Close'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
});

export const SpeakerConfidenceDialog = defineModal<
  SpeakerConfidenceDialogProps,
  SpeakerCorrectionResult | null
>(SpeakerConfidenceDialogImpl);
