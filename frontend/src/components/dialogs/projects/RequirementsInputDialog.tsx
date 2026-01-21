import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Label } from '@/components/ui/label';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { AlertCircle, Loader2, Sparkles, FileText, X } from 'lucide-react';
import { requirementsApi } from '@/lib/api';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal } from '@/lib/modals';

export interface RequirementsInputDialogProps {
  projectId: string;
  projectName: string;
}

export type RequirementsInputDialogResult =
  | { status: 'submitted'; requirementsId: string }
  | { status: 'skipped' }
  | { status: 'canceled' };

const RequirementsInputDialogImpl = NiceModal.create<RequirementsInputDialogProps>(
  ({ projectId, projectName }) => {
    const modal = useModal();
    const [requirements, setRequirements] = useState('');
    const [prdContent, setPrdContent] = useState('');
    const [showPrdInput, setShowPrdInput] = useState(false);
    const [error, setError] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);

    useEffect(() => {
      if (modal.visible) {
        setRequirements('');
        setPrdContent('');
        setShowPrdInput(false);
        setError('');
        setIsSubmitting(false);
      }
    }, [modal.visible]);

    const handleSubmit = async () => {
      if (!requirements.trim()) {
        setError('Please enter your project requirements');
        return;
      }

      setIsSubmitting(true);
      setError('');

      try {
        const result = await requirementsApi.create(projectId, {
          raw_requirements: requirements.trim(),
          prd_content: prdContent.trim() || null,
        });

        modal.resolve({
          status: 'submitted',
          requirementsId: result.id,
        } as RequirementsInputDialogResult);
        modal.hide();
      } catch (err) {
        setError(
          err instanceof Error
            ? err.message
            : 'Failed to analyze requirements. Please try again.'
        );
        setIsSubmitting(false);
      }
    };

    const handleSkip = () => {
      modal.resolve({ status: 'skipped' } as RequirementsInputDialogResult);
      modal.hide();
    };

    const handleCancel = () => {
      modal.resolve({ status: 'canceled' } as RequirementsInputDialogResult);
      modal.hide();
    };

    const handleOpenChange = (open: boolean) => {
      if (!open && !isSubmitting) {
        handleCancel();
      }
    };

    return (
      <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
        <DialogContent className="sm:max-w-[600px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Sparkles className="h-5 w-5 text-primary" />
              Add Requirements for {projectName}
            </DialogTitle>
            <DialogDescription>
              Describe what you want to build and AI will generate tasks for your
              kanban board. You can skip this and add tasks manually.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="requirements">
                What do you want to build?
              </Label>
              <Textarea
                id="requirements"
                value={requirements}
                onChange={(e) => setRequirements(e.target.value)}
                placeholder="Describe your project requirements, features, and goals...

Example:
- User authentication with OAuth
- Dashboard showing analytics
- Settings page for user preferences
- REST API for mobile app"
                className="min-h-[160px] resize-none"
                disabled={isSubmitting}
              />
              <p className="text-xs text-muted-foreground">
                Be as detailed as you want. The AI will extract features and
                create implementation tasks.
              </p>
            </div>

            {!showPrdInput ? (
              <button
                type="button"
                className="text-sm text-muted-foreground hover:text-foreground flex items-center gap-1"
                onClick={() => setShowPrdInput(true)}
                disabled={isSubmitting}
              >
                <FileText className="h-3 w-3" />
                Add PRD or specification document
              </button>
            ) : (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <Label htmlFor="prd">
                    PRD / Specification (optional)
                  </Label>
                  <button
                    type="button"
                    className="text-muted-foreground hover:text-foreground"
                    onClick={() => {
                      setShowPrdInput(false);
                      setPrdContent('');
                    }}
                    disabled={isSubmitting}
                  >
                    <X className="h-4 w-4" />
                  </button>
                </div>
                <Textarea
                  id="prd"
                  value={prdContent}
                  onChange={(e) => setPrdContent(e.target.value)}
                  placeholder="Paste your PRD, technical spec, or any additional documentation..."
                  className="min-h-[120px] resize-none"
                  disabled={isSubmitting}
                />
              </div>
            )}

            {error && (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
          </div>

          <DialogFooter className="gap-2 sm:gap-0">
            <Button
              type="button"
              variant="ghost"
              onClick={handleSkip}
              disabled={isSubmitting}
            >
              Skip for now
            </Button>
            <Button
              type="button"
              onClick={handleSubmit}
              disabled={isSubmitting || !requirements.trim()}
            >
              {isSubmitting ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  Analyzing...
                </>
              ) : (
                <>
                  <Sparkles className="h-4 w-4 mr-2" />
                  Generate Tasks
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const RequirementsInputDialog = defineModal<
  RequirementsInputDialogProps,
  RequirementsInputDialogResult
>(RequirementsInputDialogImpl);
