import { cn } from '@/lib/utils';
import { Slider as SliderPrimitive } from '@base-ui/react/slider';

type SliderProps<Value extends number | readonly number[]> = SliderPrimitive.Root.Props<Value>;

function Slider<Value extends number | readonly number[]>({
  className,
  defaultValue,
  value,
  min = 0,
  max = 100,
  ...props
}: SliderProps<Value>) {
  const values =
    value !== undefined
      ? Array.isArray(value)
        ? value
        : [value]
      : defaultValue !== undefined
        ? Array.isArray(defaultValue)
          ? defaultValue
          : [defaultValue]
        : [min];

  return (
    <SliderPrimitive.Root
      data-slot="slider"
      className={cn('w-full', className)}
      defaultValue={defaultValue}
      value={value}
      min={min}
      max={max}
      thumbAlignment="center"
      {...props}
    >
      <SliderPrimitive.Control className="relative flex h-8 w-full items-center touch-none select-none">
        <SliderPrimitive.Track
          data-slot="slider-track"
          className="relative h-2 w-full overflow-hidden rounded-full bg-white/10 ring-1 ring-white/8"
        >
          <SliderPrimitive.Indicator
            data-slot="slider-range"
            className="absolute inset-y-0 left-0 rounded-full bg-primary"
          />
        </SliderPrimitive.Track>

        {values.map((_, index) => (
          <SliderPrimitive.Thumb
            key={index}
            data-slot="slider-thumb"
            className="block size-5 rounded-full border-2 border-white bg-primary shadow-[0_0_0_4px_rgba(255,45,45,0.18)] transition-transform outline-none hover:scale-105 focus-visible:scale-105 focus-visible:ring-4 focus-visible:ring-red-500/20 disabled:pointer-events-none disabled:opacity-50"
          />
        ))}
      </SliderPrimitive.Control>
    </SliderPrimitive.Root>
  );
}

export { Slider };
