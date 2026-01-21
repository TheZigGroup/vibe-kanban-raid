import { useCallback } from 'react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Sparkles,
  FileText,
  Loader2,
  CheckCircle2,
  AlertCircle,
  ChevronDown,
  Trash2,
} from 'lucide-react';
import { useProjectRequirements } from '@/hooks/useProjectRequirements';
import { RequirementsInputDialog } from '@/components/dialogs/projects/RequirementsInputDialog';
import type { GenerationStatus } from 'shared/types';

interface ProjectRequirementsSectionProps {
  projectId: string;
  projectName: string;
}

const STATUS_LABELS: Record<GenerationStatus, string> = {
  pending: 'Processing requirements...',
  analyzing: 'Analyzing requirements...',
  generating: 'Generating tasks...',
  completed: 'Tasks generated',
  failed: 'Generation failed',
};

export function ProjectRequirementsSection({
  projectId,
  projectName,
}: ProjectRequirementsSectionProps) {
  const {
    requirements,
    isLoading,
    deleteRequirements,
    isDeleting,
  } = useProjectRequirements(projectId);

  const handleAddRequirements = useCallback(async () => {
    const result = await RequirementsInputDialog.show({
      projectId,
      projectName,
    });

    if (result.status === 'submitted') {
      // Requirements submitted, the hook will start polling for status
    }
  }, [projectId, projectName]);

  const handleDeleteRequirements = useCallback(async () => {
    if (confirm('Are you sure you want to delete the requirements? This will not delete the generated tasks.')) {
      await deleteRequirements();
    }
  }, [deleteRequirements]);

  if (isLoading) {
    return null;
  }

  // No requirements exist - show button to add them
  if (!requirements) {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className="gap-2">
            <Sparkles className="h-4 w-4" />
            AI Tasks
            <ChevronDown className="h-3 w-3" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onClick={handleAddRequirements}>
            <FileText className="h-4 w-4 mr-2" />
            Add Requirements
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    );
  }

  const status = requirements.generation_status;

  // Generation in progress
  if (status === 'pending' || status === 'analyzing' || status === 'generating') {
    return (
      <div className="flex items-center gap-2 px-3 py-1.5 bg-muted rounded-md">
        <Loader2 className="h-4 w-4 animate-spin text-primary" />
        <span className="text-sm text-muted-foreground">
          {STATUS_LABELS[status]}
        </span>
      </div>
    );
  }

  // Generation failed
  if (status === 'failed') {
    return (
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className="gap-2 border-destructive text-destructive">
            <AlertCircle className="h-4 w-4" />
            Generation Failed
            <ChevronDown className="h-3 w-3" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onClick={handleAddRequirements}>
            <Sparkles className="h-4 w-4 mr-2" />
            Retry Generation
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={handleDeleteRequirements}
            className="text-destructive"
            disabled={isDeleting}
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Delete Requirements
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    );
  }

  // Generation completed - show status and block re-generation
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <CheckCircle2 className="h-4 w-4 text-green-600" />
          {requirements.tasks_generated} Tasks Generated
          <ChevronDown className="h-3 w-3" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <div className="px-2 py-1.5 text-xs text-muted-foreground">
          Tasks already generated from requirements.
          <br />
          Delete to regenerate with new requirements.
        </div>
        <DropdownMenuItem
          onClick={handleDeleteRequirements}
          className="text-destructive"
          disabled={isDeleting}
        >
          <Trash2 className="h-4 w-4 mr-2" />
          Delete Requirements
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
