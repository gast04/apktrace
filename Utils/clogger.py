import os
import functools
import logging
import time
from datetime import datetime
from pathlib import Path
from typing import Callable, Any, Tuple


class CustomFormatter(logging.Formatter):
    def __init__(self, fmt=None, datefmt=None, id="", style="%", prefix="ROOT"):
        super().__init__(fmt, datefmt, style)  # type: ignore
        self.prefix = prefix
        self.id = id

    def format(self, record):
        record.prefix = self.prefix
        record.id = self.id
        record.asctime = datetime.now().strftime(self.datefmt)  # type: ignore
        return super().format(record)


def create_logger(
    logger_name: str,
    prefix: str,
    id: str = "",
    loglevel: int | None = None,
    filepath: Path | None = None,
) -> logging.Logger:
    full_logger_name = f"{logger_name}{id}"

    # if the logger already exists we can just return it, adding another handler
    # would lead to double messages
    if full_logger_name in logging.Logger.manager.loggerDict:
        return logging.getLogger(full_logger_name)

    # create new logger
    fmt = f'%(prefix)s | {"%(id)s | " if id else ""}%(levelname)-7s | %(asctime)s.%(msecs)03d | %(message)s'
    formatter = CustomFormatter(
        fmt=fmt,
        datefmt="%Y-%m-%d %H:%M:%S",
        prefix=prefix,
        id=id,
    )

    logger = logging.getLogger(full_logger_name)
    logger.propagate = False

    # always create streamhandler, to have logs easily accessible in dozzle
    stream_handler = logging.StreamHandler()
    stream_handler.setFormatter(formatter)
    logger.addHandler(stream_handler)
    if filepath is not None:
        file_handler = logging.FileHandler(filepath / f"{full_logger_name}.log")
        file_handler.setFormatter(formatter)
        logger.addHandler(file_handler)

    logger.setLevel(loglevel if loglevel else logging.DEBUG)
    return logger


def log_execution_time(func: Callable[..., Any]) -> Callable[..., Any]:
    @functools.wraps(func)
    def wrapper(*args: Tuple[Any, ...], **kwargs: Any) -> Any:
        start_time = time.time()
        result = func(*args, **kwargs)
        exec_time = time.time() - start_time

        """
        minutes, seconds = divmod(exec_time, 60)
        if AppConfig.loglevel == logging.DEBUG:
            # Cannot create a logger here
            print(f"Execution time: {func.__name__}, {int(minutes)}min {seconds:.2f}s")
        """
        return result

    return wrapper

