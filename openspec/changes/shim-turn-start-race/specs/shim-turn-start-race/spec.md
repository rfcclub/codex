# Requirements

## Superseding an idle reservation

WHEN a session has reserved an idle turn but has not attached a running task,
AND a replacement turn begins,
THEN the replacement clears the idle reservation.

## Completing a superseded startup

WHEN a task startup resumes after its reservation was superseded,
THEN it exits without attaching a task or panicking.
