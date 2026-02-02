import { motion, HTMLMotionProps } from 'framer-motion'
import { cn } from '@/lib/utils'

interface MotionProps extends HTMLMotionProps<"div"> {
  delay?: number
  duration?: number
}

export const FadeIn = ({ 
  children, 
  className, 
  delay = 0, 
  duration = 0.5,
  ...props 
}: MotionProps) => {
  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: 10 }}
      transition={{ duration, delay, ease: "easeOut" }}
      className={cn(className)}
      {...props}
    >
      {children}
    </motion.div>
  )
}

export const SlideIn = ({ 
  children, 
  className, 
  delay = 0, 
  duration = 0.5,
  direction = "left",
  ...props 
}: MotionProps & { direction?: "left" | "right" | "up" | "down" }) => {
  const variants = {
    hidden: { 
      opacity: 0,
      x: direction === "left" ? -20 : direction === "right" ? 20 : 0,
      y: direction === "up" ? 20 : direction === "down" ? -20 : 0
    },
    visible: { 
      opacity: 1, 
      x: 0, 
      y: 0 
    }
  }

  return (
    <motion.div
      initial="hidden"
      animate="visible"
      exit="hidden"
      variants={variants}
      transition={{ duration, delay, ease: "easeOut" }}
      className={cn(className)}
      {...props}
    >
      {children}
    </motion.div>
  )
}

export const ScaleIn = ({ 
  children, 
  className, 
  delay = 0, 
  duration = 0.3,
  ...props 
}: MotionProps) => {
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ duration, delay, ease: "easeOut" }}
      className={cn(className)}
      {...props}
    >
      {children}
    </motion.div>
  )
}
