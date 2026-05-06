/* Function referenced only via an ALL-CAPS-named array — not dead */
typedef void (*Handler) (void);

static void
handle_activate (void)
{
}

static Handler HANDLERS[] = {
  handle_activate,
};
