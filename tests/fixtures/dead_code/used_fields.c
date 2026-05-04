typedef struct {
    int x;
    int y;
} Point;

static int
distance_sq (Point *a, Point *b)
{
    int dx = a->x - b->x;
    int dy = a->y - b->y;
    return dx * dx + dy * dy;
}
