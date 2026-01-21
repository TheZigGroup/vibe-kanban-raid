import { Bot, Loader2, Play, CheckCircle2 } from 'lucide-react';
import { useAgentActivity } from '@/hooks';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  TooltipProvider,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';

interface AgentActivityToggleProps {
  projectId: string;
  className?: string;
}

export function AgentActivityToggle({
  projectId,
  className,
}: AgentActivityToggleProps) {
  const {
    status,
    isLoading,
    isEnabled,
    enable,
    disable,
    trigger,
    isEnabling,
    isDisabling,
    isTriggering,
  } = useAgentActivity(projectId);

  const isToggling = isEnabling || isDisabling;

  const handleToggle = async (checked: boolean) => {
    if (checked) {
      await enable();
    } else {
      await disable();
    }
  };

  const handleTrigger = async () => {
    try {
      const result = await trigger();
      if (result.action === 'selected' && result.reasoning) {
        console.log('Agent selected task:', result.reasoning);
      }
    } catch (err) {
      console.error('Failed to trigger agent activity:', err);
    }
  };

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-2', className)}>
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <TooltipProvider>
      <div className={cn('flex items-center gap-3', className)}>
        <Tooltip>
          <TooltipTrigger asChild>
            <div className="flex items-center gap-2">
              <Bot
                className={cn(
                  'h-4 w-4',
                  isEnabled ? 'text-primary' : 'text-muted-foreground'
                )}
              />
              <span className="text-sm text-muted-foreground">Auto-select</span>
              <Switch
                checked={isEnabled}
                onCheckedChange={handleToggle}
                disabled={isToggling}
                aria-label="Toggle agent activity"
              />
            </div>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            <p>
              {isEnabled
                ? 'Agent will automatically select the next task every minute'
                : 'Enable to let the agent automatically select tasks'}
            </p>
          </TooltipContent>
        </Tooltip>

        {isEnabled && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleTrigger}
                disabled={isTriggering}
                className="h-7 px-2"
              >
                {isTriggering ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <Play className="h-3 w-3" />
                )}
                <span className="ml-1 text-xs">Run now</span>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <p>Manually trigger task selection</p>
            </TooltipContent>
          </Tooltip>
        )}

        {status?.last_run && status?.last_reasoning && (
          <Tooltip>
            <TooltipTrigger asChild>
              <div className="flex items-center gap-1 text-xs text-muted-foreground">
                <CheckCircle2 className="h-3 w-3 text-green-500" />
                <span className="max-w-[150px] truncate">
                  {status.last_reasoning}
                </span>
              </div>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="max-w-xs">
              <p>{status.last_reasoning}</p>
              <p className="text-xs text-muted-foreground mt-1">
                Last run: {new Date(status.last_run).toLocaleString()}
              </p>
            </TooltipContent>
          </Tooltip>
        )}
      </div>
    </TooltipProvider>
  );
}
