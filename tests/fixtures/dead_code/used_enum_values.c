typedef enum {
    BAR_STATE_IDLE,
    BAR_STATE_RUNNING,
    BAR_STATE_DONE,
} BarState;

void
bar_update (BarState state)
{
    switch (state) {
    case BAR_STATE_IDLE:
    case BAR_STATE_RUNNING:
    case BAR_STATE_DONE:
        break;
    }
}
